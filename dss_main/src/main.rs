#![windows_subsystem = "windows"]

mod gl_utils;

#[macro_use]
extern crate glium;

use dss_mlb::MlbGameClientInfo;
use gl_utils::Vertex;
use glium::glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::{Fullscreen, WindowBuilder};
use glium::glutin::ContextBuilder;
use glium::texture::{RawImage2d, Texture2d};
use glium::{Display, Surface};
use glium_glyph::glyph_brush::Section;
use glium_glyph::GlyphBrushBuilder;
use log::error;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

const PAGE_SIZE: usize = 4;
const DEFAULT_RAW: &[u8; 22931] = include_bytes!("default.jpg");

#[tokio::main]
async fn main() {
    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} - {({l}):5.5}{n}    {m}{n}{n}",
        )))
        .build("log/dss.log")
        .expect("Unable to create log file appender.");
    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(Root::builder().appender("log_file").build(log::LevelFilter::Warn))
        .expect("Unable to create log configuration.");
    log4rs::init_config(config).expect("Unable to apply logging configuration.");

    let result = dss_mlb::get_games().await;
    let mut days = Vec::with_capacity(result.len());
    let mut focused_day: usize = 0;
    let mut focused_index: usize = 0;
    for day in result.values().rev() {
        let mut games: Vec<MlbGameGlInfo> = Vec::with_capacity(day.len());
        for game in day {
            games.push(game.to_owned().into());
        }
        days.push(DayRowInfo::new(games));
    }

    let event_loop = EventLoop::new();
    let monitor = event_loop.primary_monitor();
    let wb = WindowBuilder::new()
        .with_title("JDN DSS Solution")
        .with_inner_size(monitor.size())
        .with_fullscreen(Some(Fullscreen::Borderless(monitor)));
    let cb = ContextBuilder::new();
    let display = Display::new(wb, cb, &event_loop).unwrap_or_else(|ex| {
        let msg = "Could not create Display";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });

    let program = glium::Program::from_source(
        &display,
        gl_utils::VERTEX_SHADER_SRC,
        gl_utils::FRAGMENT_SHADER_SRC,
        None,
    )
    .unwrap_or_else(|ex| {
        let msg = "Could not create OpenGL Program";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });

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
    let background_buffer = glium::VertexBuffer::new(&display, &background_shape).unwrap_or_else(|ex| {
        let msg = "Could not create background buffer";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });

    let background_rgba = image::load_from_memory(include_bytes!("background.jpg"))
        .unwrap_or_else(|ex| {
            let msg = "Could not load background image";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        })
        .into_rgba();
    let background_dimensions = background_rgba.dimensions();
    let background_image = RawImage2d::from_raw_rgba_reversed(&background_rgba.into_raw(), background_dimensions);
    let background_texture = Texture2d::new(&display, background_image).unwrap_or_else(|ex| {
        let msg = "Could not create background texture";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });
    let background_uniforms = uniform! {
        matrix: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0 , 0.0, 0.0, 1.0f32],
        ],
        tex: &background_texture,
    };
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 0.0, 0.0);
    target
        .draw(
            &background_buffer,
            &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
            &program,
            &background_uniforms,
            &Default::default(),
        )
        .unwrap_or_else(|ex| {
            let msg = "Target could not draw background";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
    target.finish().unwrap_or_else(|ex| {
        let msg = "Target could not finish initial pass";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });

    let game_width = 0.375;
    let game_height = 0.28125;
    let game_padding = 0.05;
    let mut game_shapes = Vec::with_capacity(days.len() * PAGE_SIZE);
    for row in 0..days.len() {
        for i in 0..PAGE_SIZE {
            let x_offset = i as f32 * (game_width + game_padding * 2.0);
            let y_offset = row as f32 * (game_height + game_padding * 2.0);
            game_shapes.push(vec![
                Vertex {
                    position: [-0.95 + x_offset, 0.0 - y_offset],
                    tex_coords: [0.0, 0.0],
                },
                Vertex {
                    position: [-0.95 + x_offset, 0.28125 - y_offset],
                    tex_coords: [0.0, 1.0],
                },
                Vertex {
                    position: [-0.525 + x_offset, 0.28125 - y_offset],
                    tex_coords: [1.0, 1.0],
                },
                Vertex {
                    position: [-0.95 + x_offset, 0.0 - y_offset],
                    tex_coords: [0.0, 0.0],
                },
                Vertex {
                    position: [-0.525 + x_offset, 0.28125 - y_offset],
                    tex_coords: [1.0, 1.0],
                },
                Vertex {
                    position: [-0.525 + x_offset, 0.0 - y_offset],
                    tex_coords: [1.0, 0.0],
                },
            ]);
        }
    }
    // load text brush after first pass to prevent black screen
    let mut text_brush = GlyphBrushBuilder::using_font_bytes(include_bytes!("tahoma.ttf").to_vec()).build(&display);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let background_uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0 , 0.0, 0.0, 1.0f32],
            ],
            tex: &background_texture,
        };
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
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
                        (VirtualKeyCode::Left, ElementState::Released) => {
                            if focused_index > 0 {
                                focused_index -= 1;
                            } else if day.begin_index > 0 {
                                day.begin_index -= 1;
                            }
                        }
                        (VirtualKeyCode::Right, ElementState::Released) => {
                            if focused_index < PAGE_SIZE - 1 {
                                focused_index += 1;
                            } else if day.begin_index + PAGE_SIZE < day.games.len() {
                                day.begin_index += 1;
                            }
                        }
                        (VirtualKeyCode::Up, ElementState::Released) => {
                            if focused_day > 0 {
                                focused_day -= 1;
                            }
                        }
                        (VirtualKeyCode::Down, ElementState::Released) => {
                            if focused_day < days.len() - 1 {
                                focused_day += 1;
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target
            .draw(
                &background_buffer,
                &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                &program,
                &background_uniforms,
                &Default::default(),
            )
            .unwrap_or_else(|ex| {
                let msg = "Target could not draw background";
                error!("{}:\n{}", msg, ex);
                panic!("{}.", msg);
            });
        for (row, day) in days.iter_mut().enumerate() {
            for i in day.begin_index..(day.begin_index + PAGE_SIZE) {
                let col = i - day.begin_index;
                let game = &mut day.games[i];

                let game_shape = &game_shapes[row * PAGE_SIZE + col];

                if row == focused_day && col == focused_index {
                    let screen_dims = display.get_framebuffer_dimensions();
                    let screen_width = screen_dims.0 as f32;
                    let screen_height = screen_dims.1 as f32;
                    let text_bounds = (420.0, f32::INFINITY);

                    let game_top_left = game_shape[1].position;
                    let game_top_left_x = game_top_left[0];
                    let game_top_left_y = game_top_left[1] + game_padding;
                    let translate_x = if game_top_left_x < 0.0 {
                        0.5 - game_top_left_x * -0.5
                    } else {
                        0.5 + game_top_left_x / 2.0
                    };
                    let translate_y = if game_top_left_y < 0.0 {
                        0.5 + game_top_left_y * -0.5
                    } else {
                        0.5 - game_top_left_y / 2.0
                    };
                    let text_top_left = (translate_x * screen_width, translate_y * screen_height);
                    text_brush.queue(Section {
                        text: &game.info.title,
                        color: [1.0, 1.0, 1.0, 1.0f32],
                        screen_position: text_top_left,
                        bounds: text_bounds,
                        ..Section::default()
                    });
                    let game_bottom_left = game_shape[0].position;
                    let game_bottom_left_x = game_bottom_left[0];
                    let game_bottom_left_y = game_bottom_left[1];
                    let translate_x = if game_bottom_left_x < 0.0 {
                        0.5 - game_bottom_left_x * -0.5
                    } else {
                        0.5 + game_bottom_left_x / 2.0
                    };
                    let translate_y = if game_bottom_left_y < 0.0 {
                        0.5 + game_bottom_left_y * -0.5
                    } else {
                        0.5 - game_bottom_left_y / 2.0
                    };
                    let text_bottom_left = (translate_x * screen_width, translate_y * screen_height);
                    text_brush.queue(Section {
                        text: &game.info.summary,
                        color: [1.0, 1.0, 1.0, 1.0f32],
                        screen_position: text_bottom_left,
                        bounds: text_bounds,
                        ..Section::default()
                    });
                    text_brush.draw_queued(&display, &mut target);
                }
                let game_buffer = glium::VertexBuffer::new(&display, game_shape).unwrap_or_else(|ex| {
                    let msg = "Could not create game buffer";
                    error!("{}:\n{}", msg, ex);
                    panic!("{}.", msg);
                });
                let game_uniforms = uniform! {
                    matrix: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0f32],
                    ],
                    tex: game.get_texture(&display),
                };
                target
                    .draw(
                        &game_buffer,
                        &glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                        &program,
                        &game_uniforms,
                        &Default::default(),
                    )
                    .unwrap_or_else(|ex| {
                        let msg = "Target could not draw game";
                        error!("{}:\n{}", msg, ex);
                        panic!("{}.", msg);
                    });
            }
        }
        target.finish().unwrap_or_else(|ex| {
            let msg = "Target could not finish";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
    });
}

struct MlbGameGlInfo {
    info: MlbGameClientInfo,
    texture: Option<Texture2d>,
}

impl MlbGameGlInfo {
    pub fn get_texture(&mut self, display: &Display) -> &Texture2d {
        if self.texture.is_none() {
            let image_raw = if let Some(image) = &self.info.image {
                image.as_slice()
            } else {
                DEFAULT_RAW
            };
            let game_rgba = image::load_from_memory_with_format(image_raw, image::ImageFormat::Jpeg)
                .unwrap_or_else(|ex| {
                    let msg = "Could not create game image from bytes";
                    error!("{}:\n{}", msg, ex);
                    panic!("{}.", msg);
                })
                .into_rgba();
            let game_dimensions = game_rgba.dimensions();
            let game_image = RawImage2d::from_raw_rgba_reversed(&game_rgba.into_raw(), game_dimensions);
            let game_texture = Texture2d::new(display, game_image).unwrap_or_else(|ex| {
                let msg = "Could not create game texture";
                error!("{}:\n{}", msg, ex);
                panic!("{}.", msg);
            });
            self.texture = Some(game_texture);
        }
        self.texture.as_ref().unwrap()
    }
}

impl From<MlbGameClientInfo> for MlbGameGlInfo {
    fn from(orig: MlbGameClientInfo) -> Self {
        MlbGameGlInfo {
            info: orig,
            texture: None,
        }
    }
}

struct DayRowInfo {
    games: Vec<MlbGameGlInfo>,
    begin_index: usize,
}

impl DayRowInfo {
    pub fn new(games: Vec<MlbGameGlInfo>) -> Self {
        DayRowInfo { games, begin_index: 0 }
    }
}
