#[macro_use]
extern crate glium;

use dss_mlb::MlbGameClientInfo;
use dss_mlb::MlbManager;
use glium::glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::glutin::ContextBuilder;
use glium::texture::{RawImage2d, Texture2d};
use glium::Display;
use glium::Surface;

const PAGE_SIZE: usize = 4;

#[async_std::main]
async fn main() {
    let mlb_manager = MlbManager {};
    let result = mlb_manager.get_games().await.expect("Unable to parse JSON");
    let mut days = Vec::with_capacity(result.len());
    let mut focused_day: usize = result.len() / 2;
    for day in result {
        days.push(DayRowInfo::new(day.clone()));
    }

    let el = EventLoop::new();
    let wb = WindowBuilder::new();
    let cb = ContextBuilder::new();
    let display = Display::new(wb, cb, &el).unwrap();

    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        tex_coords: [f32; 2],
    }
    implement_vertex!(Vertex, position, tex_coords);

    let vertex_shader_src = r#"
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

    let fragment_shader_src = r#"
        #version 140

        uniform sampler2D tex;

        in vec2 v_tex_coords;
        out vec4 color;
        
        void main() {
            color = texture(tex, v_tex_coords);
        }
    "#;

    let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

    let background_rgba = image::load_from_memory(include_bytes!("background.jpg"))
        .unwrap()
        .into_rgba();
    let background_dimensions = background_rgba.dimensions();
    let background_image = RawImage2d::from_raw_rgba_reversed(&background_rgba.into_raw(), background_dimensions);
    let background_texture = Texture2d::new(&display, background_image).unwrap();
    let background_shape = vec![
        Vertex {
            position: [-1.0, -1.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [-1.0, 1.0],
            tex_coords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [-1.0, -1.0],
            tex_coords: [0.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, -1.0],
            tex_coords: [1.0, 0.0],
        },
    ];
    let background_buffer = glium::VertexBuffer::new(&display, &background_shape).unwrap();

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
                } => {
                    let day = &mut days[focused_day];
                    match (virtual_code, state) {
                        (VirtualKeyCode::Escape, _) => {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                        (VirtualKeyCode::Left, ElementState::Released) => {
                            day.move_left();
                        }
                        (VirtualKeyCode::Right, ElementState::Released) => {
                            day.move_right();
                        }
                        (VirtualKeyCode::Up, ElementState::Released) => {
                            if focused_day > 0 {
                                focused_day -= 1;
                            }
                        }
                        (VirtualKeyCode::Down, ElementState::Released) => {
                            if focused_day < days.len() - 2 {
                                focused_day += 1;
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            },
            Event::NewEvents(cause) => match cause {
                glium::glutin::event::StartCause::ResumeTimeReached { .. } => (),
                glium::glutin::event::StartCause::Init => (),
                _ => return,
            },
            _ => {}
        }

        let next_frame_time = std::time::Instant::now() + std::time::Duration::from_nanos(33_666_667);
        *control_flow = ControlFlow::WaitUntil(next_frame_time);

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);

        let background_uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0 , 0.0, 0.0, 1.0f32],
            ],
            tex: &background_texture,
        };
        target
            .draw(
                &background_buffer,
                &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &program,
                &background_uniforms,
                &Default::default(),
            )
            .expect("Target could not draw background");

        for (y, day) in days.iter().enumerate() {
            for (x, game) in day.games.iter().enumerate() {
                if let Some(image) = &game.image {
                    let game_rgba = image::load_from_memory_with_format(image.as_slice(), image::ImageFormat::Jpeg)
                        .expect("Unable to convert game image to rgba")
                        .into_rgba();
                    let game_dimensions = game_rgba.dimensions();
                    let game_image = RawImage2d::from_raw_rgba_reversed(&game_rgba.into_raw(), game_dimensions);
                    let game_texture = Texture2d::new(&display, game_image).unwrap();
                    // width  = 0.375
                    // height = 0.28125
                    // padding = 0.05
                    let game_shape = vec![
                        Vertex {
                            position: [-0.95 + (x as f32 * 0.475), 0.0 - (y as f32 * 0.33125)],
                            tex_coords: [0.0, 0.0],
                        },
                        Vertex {
                            position: [-0.95 + (x as f32 * 0.475), 0.28125 - (y as f32 * 0.33125)],
                            tex_coords: [0.0, 1.0],
                        },
                        Vertex {
                            position: [-0.525 + (x as f32 * 0.475), 0.28125 - (y as f32 * 0.33125)],
                            tex_coords: [1.0, 1.0],
                        },
                        Vertex {
                            position: [-0.95 + (x as f32 * 0.475), 0.0 - (y as f32 * 0.33125)],
                            tex_coords: [0.0, 0.0],
                        },
                        Vertex {
                            position: [-0.525 + (x as f32 * 0.475), 0.28125 - (y as f32 * 0.33125)],
                            tex_coords: [1.0, 1.0],
                        },
                        Vertex {
                            position: [-0.525 + (x as f32 * 0.475), 0.0 - (y as f32 * 0.33125)],
                            tex_coords: [1.0, 0.0],
                        },
                    ];
                    let game_buffer = glium::VertexBuffer::new(&display, &game_shape).unwrap();
                    let game_uniforms = uniform! {
                        matrix: [
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0 , 0.0, 0.0, 1.0f32],
                        ],
                        tex: &game_texture,
                    };
                    target
                        .draw(
                            &game_buffer,
                            &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                            &program,
                            &game_uniforms,
                            &Default::default(),
                        )
                        .expect("Target could not draw game");
                }
            }
        }

        target.finish().expect("Target could not finish");
    });
}

struct DayRowInfo {
    games: Vec<MlbGameClientInfo>,
    focused_index: usize,
}

impl DayRowInfo {
    pub fn new(games: Vec<MlbGameClientInfo>) -> Self {
        DayRowInfo {
            games,
            focused_index: 0,
        }
    }

    pub fn move_left(&mut self) {
        if self.focused_index > 0 {
            self.focused_index -= 1;
            println!("{}", self.games[self.focused_index].title);
        }
    }

    pub fn move_right(&mut self) {
        if !self.games.is_empty() && self.focused_index < self.games.len() - 2 {
            self.focused_index += 1;
            println!("{}", self.games[self.focused_index].title);
        }
    }
}
