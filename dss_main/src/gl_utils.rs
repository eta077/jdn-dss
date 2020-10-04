//! General purpose OpenGL utilities.

use std::borrow::Cow;
use std::ops::Deref;

use glium::backend::{Context, Facade};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::texture2d::Texture2d;
use glium::texture::{ClientFormat, RawImage2d};
use glium::{Blend, DrawParameters, Frame, Program, Surface, VertexBuffer};
use glyph_brush::ab_glyph::FontArc;
use glyph_brush::{BrushAction, BrushError, Extra, Section};
use log::error;
use rusttype::{point, Rect};

/// The vertex shader program used to render an image.
pub const IMAGE_VERTEX_SHADER_SRC: &str = r#"
    #version 140

    uniform mat4 matrix;

    in vec2 position;
    in vec2 tex_coords;

    out vec2 v_tex_coords;
            
    void main() {
        v_tex_coords = tex_coords;
        gl_Position = matrix * vec4(position, 0.0, 1.0);
    }
"#;

/// The fragment shader program used to render an image.
pub const IMAGE_FRAGMENT_SHADER_SRC: &str = r#"
    #version 140

    uniform sampler2D tex;

    in vec2 v_tex_coords;

    out vec4 color;
    
    void main() {
        color = texture(tex, v_tex_coords);
    }
"#;

/// A container for the position of a vertex and the associated texture.
#[derive(Copy, Clone)]
pub struct ImageVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}
implement_vertex!(ImageVertex, position, tex_coords);

/// The vertex shader program used to render a point with a color.
pub const RECT_VERTEX_SHADER_SRC: &str = r#"
    #version 140

    uniform mat4 matrix;

    in vec2 position;
    in vec4 color;

    out vec4 f_color;

    void main() {
        f_color = color;
        gl_Position = matrix * vec4(position, 0.0, 1.0);
    }
"#;

/// The fragment shader program used to apply the color of a vertex.
pub const RECT_FRAGMENT_SHADER_SRC: &str = r#"
    #version 140
    
    in vec4 f_color;

    out vec4 color;

    void main() {
        color = f_color;
    }
"#;

/// A container for the position and color of a vertex.
#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}
implement_vertex!(Vertex, position, color);

/// The vertex shader program used to render a glyph.
pub const GLYPH_VERTEX_SHADER_SRC: &str = r#"
    #version 150

    const mat4 INVERT_Y_AXIS = mat4(
        vec4(1.0, 0.0, 0.0, 0.0),
        vec4(0.0, -1.0, 0.0, 0.0),
        vec4(0.0, 0.0, 1.0, 0.0),
        vec4(0.0, 0.0, 0.0, 1.0)
    );

    uniform mat4 transform;

    in vec3 left_top;
    in vec2 right_bottom;
    in vec2 tex_left_top;
    in vec2 tex_right_bottom;
    in vec4 color;

    out vec2 f_tex_pos;
    out vec4 f_color;

    // generate positional data based on vertex ID
    void main() {
        vec2 pos = vec2(0.0);
        float left = left_top.x;
        float right = right_bottom.x;
        float top = left_top.y;
        float bottom = right_bottom.y;

        switch (gl_VertexID) {
            case 0:
                pos = vec2(left, top);
                f_tex_pos = tex_left_top;
                break;
            case 1:
                pos = vec2(right, top);
                f_tex_pos = vec2(tex_right_bottom.x, tex_left_top.y);
                break;
            case 2:
                pos = vec2(left, bottom);
                f_tex_pos = vec2(tex_left_top.x, tex_right_bottom.y);
                break;
            case 3:
                pos = vec2(right, bottom);
                f_tex_pos = tex_right_bottom;
                break;
        }

        f_color = color;
        gl_Position = INVERT_Y_AXIS * transform * vec4(pos, left_top.z, 1.0);
    }
"#;

/// The fragment shader program used to render a glyph.
pub const GLYPH_FRAGMENT_SHADER_SRC: &str = r#"
    #version 150

    uniform sampler2D font_tex;

    in vec2 f_tex_pos;
    in vec4 f_color;

    out vec4 Target0;

    void main() {
        float alpha = texture(font_tex, f_tex_pos).r;
        if (alpha <= 0.0) {
            discard;
        }
        Target0 = f_color * vec4(1.0, 1.0, 1.0, alpha);
    }
"#;

#[derive(Copy, Clone, Debug)]
pub struct GlyphVertex {
    /// screen position
    pub left_top: [f32; 3],
    pub right_bottom: [f32; 2],
    /// texture position
    pub tex_left_top: [f32; 2],
    pub tex_right_bottom: [f32; 2],
    /// text color
    pub color: [f32; 4],
}
implement_vertex!(
    GlyphVertex,
    left_top,
    right_bottom,
    tex_left_top,
    tex_right_bottom,
    color
);

#[derive(Copy, Clone, Debug)]
pub struct InstanceVertex {
    pub v: f32,
}
implement_vertex!(InstanceVertex, v);

/// A "practically collision free" `Section` hasher
#[cfg(not(target_arch = "wasm32"))]
pub type DefaultSectionHasher = twox_hash::RandomXxHashBuilder;
// Work around for rand issues in wasm #61
#[cfg(target_arch = "wasm32")]
pub type DefaultSectionHasher = std::hash::BuildHasherDefault<twox_hash::XxHash>;

fn rect_to_rect(rect: Rect<u32>) -> glium::Rect {
    glium::Rect {
        left: rect.min.x,
        bottom: rect.min.y,
        width: rect.width(),
        height: rect.height(),
    }
}

fn rect_from_rect(rect: glyph_brush::Rectangle<u32>) -> Rect<u32> {
    Rect {
        min: rusttype::Point {
            x: rect.min[0],
            y: rect.min[1],
        },
        max: rusttype::Point {
            x: rect.max[0],
            y: rect.max[1],
        },
    }
}

fn update_texture(tex: &Texture2d, rect: Rect<u32>, tex_data: &[u8]) {
    let image = RawImage2d {
        data: Cow::Borrowed(tex_data),
        format: ClientFormat::U8,
        height: rect.height(),
        width: rect.width(),
    };
    tex.write(rect_to_rect(rect), image);
}

#[inline]
fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        extra,
    }: glyph_brush::GlyphVertex,
) -> GlyphVertex {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
        max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    GlyphVertex {
        left_top: [gl_rect.min.x, gl_rect.max.y, extra.z],
        right_bottom: [gl_rect.max.x, gl_rect.min.y],
        tex_left_top: [tex_coords.min.x, tex_coords.max.y],
        tex_right_bottom: [tex_coords.max.x, tex_coords.min.y],
        color: extra.color,
    }
}

pub struct GlyphBrush<'a> {
    glyph_brush: glyph_brush::GlyphBrush<GlyphVertex, Extra, FontArc, DefaultSectionHasher>,
    params: DrawParameters<'a>,
    program: Program,
    texture: Texture2d,
    index_buffer: NoIndices,
    vertex_buffer: glium::VertexBuffer<GlyphVertex>,
    instances: glium::VertexBuffer<InstanceVertex>,
}

impl<'a> GlyphBrush<'a> {
    pub fn build<F: Facade>(font: FontArc, display: &F) -> GlyphBrush<'a> {
        let params = DrawParameters {
            blend: Blend::alpha_blending(),
            ..Default::default()
        };
        let glyph_brush = glyph_brush::GlyphBrushBuilder::using_fonts(vec![font]).build();
        let (cache_width, cache_height) = glyph_brush.texture_dimensions();
        let program = Program::from_source(display, GLYPH_VERTEX_SHADER_SRC, GLYPH_FRAGMENT_SHADER_SRC, None)
            .unwrap_or_else(|ex| {
                let msg = "Could not load glyph program";
                error!("{}:\n{}", msg, ex);
                panic!("{}.", msg);
            });
        let texture = Texture2d::empty(display, cache_width, cache_height).unwrap();
        let index_buffer = NoIndices(PrimitiveType::TriangleStrip);
        // We only need this so that we have groups of four
        // instances each which is what the shader expects.
        // Dunno if there is a nicer way to do this than this
        // hack.
        let instances = VertexBuffer::new(display, &[InstanceVertex { v: 0.0 }; 4]).unwrap();
        let vertex_buffer = VertexBuffer::empty(display, 0).unwrap();
        GlyphBrush {
            glyph_brush,
            params,
            program,
            texture,
            index_buffer,
            vertex_buffer,
            instances,
        }
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue(&mut self, section: Section) {
        self.glyph_brush.queue(section)
    }

    #[inline]
    pub fn draw_queued<F: Facade + Deref<Target = Context>>(&mut self, facade: &F, frame: &mut Frame) {
        let dims = facade.get_framebuffer_dimensions();
        let transform = [
            [2.0 / (dims.0 as f32), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (dims.1 as f32), 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, -1.0, 0.0, 1.0],
        ];
        let mut brush_action;
        loop {
            // We need this scope because of lifetimes.
            // Ultimately, we'd like to put the &self.texture
            // into the closure, but that'd inevitably
            // borrow the entirety of self inside the closure.
            // This is a problem with the language and is
            // discussed here:
            // http://smallcultfollowing.com/babysteps/blog/2018/11/01/after-nll-interprocedural-conflicts/
            {
                let tex = &self.texture;
                brush_action = self.glyph_brush.process_queued(
                    |rect, tex_data| {
                        update_texture(tex, rect_from_rect(rect), tex_data);
                    },
                    to_vertex,
                );
            }
            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested }) => {
                    let (nwidth, nheight) = suggested;
                    self.texture = Texture2d::empty(facade, nwidth, nheight).unwrap();
                    // SHOULD BE AVOIDED IF POSSIBLE
                    self.glyph_brush.resize_texture(nwidth, nheight);
                }
            }
        }

        let sampler = glium::uniforms::Sampler::new(&self.texture)
            .wrap_function(glium::uniforms::SamplerWrapFunction::Clamp)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Linear)
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear);

        match brush_action.unwrap() {
            BrushAction::Draw(verts) => {
                self.vertex_buffer = glium::VertexBuffer::new(facade, &verts).unwrap();
            }
            BrushAction::ReDraw => {}
        };

        let uniforms = uniform! {
            font_tex: sampler,
            transform: transform,
        };

        // drawing a frame
        frame
            .draw(
                (&self.instances, self.vertex_buffer.per_instance().unwrap()),
                &self.index_buffer,
                &self.program,
                &uniforms,
                &self.params,
            )
            .unwrap();
    }
}

/// An enumeration of directions in which focus can move.
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}
