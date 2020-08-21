#[macro_use]
extern crate glium;

use glium::glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::glutin::ContextBuilder;
use glium::texture::{RawImage2d, Texture2d};
use glium::Display;
use glium::Surface;

pub fn main() {
    let background = image::load_from_memory(include_bytes!("background.jpg"))
        .unwrap()
        .into_rgba();
    let background_dimensions = background.dimensions();
    let background_image = RawImage2d::from_raw_rgba_reversed(&background.into_raw(), background_dimensions);

    let el = EventLoop::new();
    let wb = WindowBuilder::new();
    let cb = ContextBuilder::new();
    let display = Display::new(wb, cb, &el).unwrap();
    let background_texture = Texture2d::new(&display, background_image).unwrap();

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        tex_coords: [f32; 2],
    }

    implement_vertex!(Vertex, position, tex_coords);

    let vertex1 = Vertex {
        position: [0.0, -1.0],
        tex_coords: [0.0, 0.0],
    };
    let vertex2 = Vertex {
        position: [0.0, 1.0],
        tex_coords: [0.0, 1.0],
    };
    let vertex3 = Vertex {
        position: [2.0, 1.0],
        tex_coords: [1.0, 1.0],
    };
    let vertex4 = Vertex {
        position: [0.0, -1.0],
        tex_coords: [0.0, 0.0],
    };
    let vertex5 = Vertex {
        position: [2.0, 1.0],
        tex_coords: [1.0, 1.0],
    };
    let vertex6 = Vertex {
        position: [2.0, -1.0],
        tex_coords: [1.0, 0.0],
    };
    let shape = vec![
        vertex1, vertex2, vertex3, 
        vertex4, vertex5, vertex6
        ];

    let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

    let vertex_shader_src = r#"
        #version 140

        in vec2 position;
        in vec2 tex_coords;
        out vec2 v_tex_coords;
        
        uniform mat4 matrix;
        
        void main() {
            v_tex_coords = tex_coords;
            gl_Position = matrix * vec4(position, 0.0, 1.0);
        }
    "#;

    let fragment_shader_src = r#"
        #version 140

        in vec2 v_tex_coords;
        out vec4 color;
        
        uniform sampler2D tex;
        
        void main() {
            color = texture(tex, v_tex_coords);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();
    let mut page = 1;
    let mut games = vec![1, 2, 3];
    let mut focused_index = 0;

    el.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_code),
                            state,
                            ..
                        },
                    ..
                } => match (virtual_code, state) {
                    (VirtualKeyCode::Escape, _) => *control_flow = ControlFlow::Exit,
                    (VirtualKeyCode::Left, ElementState::Released) => {
                        if focused_index > 0 {
                            focused_index -= 1;
                            println!("{}", focused_index);
                        }
                    }
                    (VirtualKeyCode::Right, ElementState::Released) => {
                        if focused_index < games.len() - 1 {
                            focused_index += 1;
                            println!("{}", focused_index);
                        }
                    }
                    _ => (),
                },
                _ => (),
            },
            Event::NewEvents(cause) => match cause {
                glium::glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glium::glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => {}
        }

        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = ControlFlow::WaitUntil(next_frame_time);

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0 , 0.0, 0.0, 1.0f32],
            ],
            tex: &background_texture,
        };
        target
            .draw(&vertex_buffer, &indices, &program, &uniforms, &Default::default())
            .unwrap();
        target.finish().unwrap();
    });
}
