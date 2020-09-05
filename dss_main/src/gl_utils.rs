//! General purpose OpenGL utilities.

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

/// An enumeration of directions in which focus can move.
pub enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}
