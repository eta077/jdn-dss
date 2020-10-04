#![windows_subsystem = "windows"]

//! OpenGL implementation of the DSS UI.

mod gl_mlb;
mod gl_utils;

#[macro_use]
extern crate glium;

use gl_mlb::{MlbGlUi, MlbUiInfo};
use gl_utils::FocusDirection;
use glium::glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::{Fullscreen, WindowBuilder};
use glium::glutin::ContextBuilder;
use glium::{Display, Surface};
use glyph_brush::ab_glyph::FontArc;
use log::{error, info};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

#[tokio::main]
async fn main() {
    // setup logging
    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} - {({l}):5.5}{n}    {m}{n}{n}",
        )))
        .build("log/dss.log")
        .expect("Unable to create log file appender.");
    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(Root::builder().appender("log_file").build(log::LevelFilter::Info))
        .expect("Unable to create log configuration.");
    log4rs::init_config(config).expect("Unable to apply logging configuration.");

    info!("starting application");

    // load backing data
    let mlb_ui_info = MlbUiInfo::init().await;
    info!("data loaded");

    // initialize window/display
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
    info!("display created");

    // initialize individual UIs
    let mut mlb_gl = MlbGlUi::init(mlb_ui_info, &display);
    info!("MLB GUI initialized");

    // first pass before event loop
    let mut target = display.draw();
    target.clear_color(0.0, 0.0, 0.0, 0.0);
    mlb_gl.draw(&display, &mut target, None);
    target.finish().unwrap_or_else(|ex| {
        let msg = "Target could not finish initial pass";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });
    info!("first pass drawn");

    // load text brush after first pass to prevent black screen
    let font = FontArc::try_from_slice(include_bytes!("tahoma.ttf")).unwrap_or_else(|ex| {
        let msg = "Could not load font";
        error!("{}:\n{}", msg, ex);
        panic!("{}.", msg);
    });
    info!("font loaded");
    let mut text_brush = gl_utils::GlyphBrush::build(font, &display);
    info!("text brush built");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

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
                } => match (virtual_code, state) {
                    (VirtualKeyCode::Left, ElementState::Released) => mlb_gl.move_focus(FocusDirection::Left),
                    (VirtualKeyCode::Right, ElementState::Released) => mlb_gl.move_focus(FocusDirection::Right),
                    (VirtualKeyCode::Up, ElementState::Released) => mlb_gl.move_focus(FocusDirection::Up),
                    (VirtualKeyCode::Down, ElementState::Released) => mlb_gl.move_focus(FocusDirection::Down),
                    _ => (),
                },
                _ => (),
            }
        }
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        mlb_gl.draw(&display, &mut target, Some(&mut text_brush));

        target.finish().unwrap_or_else(|ex| {
            let msg = "Target could not finish";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
    });
}
