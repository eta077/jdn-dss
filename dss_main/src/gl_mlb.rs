//! OpenGL implementation of the MLB UI.

use crate::gl_utils;
use crate::gl_utils::{FocusDirection, ImageVertex, Vertex};
use dss_mlb::MlbGameClientInfo;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{RawImage2d, Texture2d};
use glium::{Display, DrawParameters, Frame, Program, Surface, VertexBuffer};
use glium_glyph::glyph_brush::Section;
use glium_glyph::GlyphBrush;
use log::error;

/// The bytes for the image to use for a game if one cannot be retrieved.
const DEFAULT_RAW: &[u8; 22931] = include_bytes!("default.jpg");
/// The number of games to display at a time for each day.
const X_PAGE_SIZE: usize = 5;
/// The percentage of the screen taken up by the border of a focused game.
const BORDER_SCALE: f32 = 0.175;
/// The percentage of the screen taken up by a line in the border of a focused game.
const BORDER_LINE_SCALE: f32 = 0.015;
/// The percentage of the screen for horizontal and vertical padding from the focused game image.
const BORDER_OFFSET: f32 = 0.025;
/// The percentage from the left of the screen at which to start displaying game images.
const LEFT_INDENT: f32 = 0.05;
/// The percentage from the top of the screen at which to start displaying game images.
const TOP_INDENT: f32 = 0.24;
/// The percentage of the screen taken up by a focused game image.
const FOCUSED_GAME_SCALE: f32 = 0.15;
/// The percentage of the screen for horizontal spacing between game images (assuming both are focused).
const GAME_X_PADDING: f32 = 0.0375;
/// The percentage of the screen for vertical spacing between game images (assuming both are focused).
const GAME_Y_PADDING: f32 = 0.05;
/// The percentage of the screen taken up by a non-focused game image.
const GAME_SCALE: f32 = 0.10;
/// The percentage of the screen added to horizontal and vertical padding to account for non-focused images.
const NON_FOCUSED_OFFSET: f32 = 0.025;

/// The manager of the MLB UI responsible for rendering implementation and ownership of the backing data.
pub struct MlbGlUi {
    ui_info: MlbUiInfo,
    image_program: Program,
    image_square_vertices: VertexBuffer<ImageVertex>,
    background_texture: Texture2d,
    rect_program: Program,
    border_vertices: VertexBuffer<Vertex>,
}

impl MlbGlUi {
    /// Initializes the MLB UI manager with the given information.
    ///
    /// # Errors
    /// Panics if the given display cannot be used to create UI elements.
    pub fn init(ui_info: MlbUiInfo, display: &Display) -> Self {
        let image_program = Program::from_source(
            display,
            gl_utils::IMAGE_VERTEX_SHADER_SRC,
            gl_utils::IMAGE_FRAGMENT_SHADER_SRC,
            None,
        )
        .unwrap_or_else(|ex| {
            let msg = "Could not create OpenGL image program";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
        let image_square_shape = vec![
            ImageVertex {
                position: [-1.0, -1.0],
                tex_coords: [0.0, 0.0],
            },
            ImageVertex {
                position: [-1.0, 1.0],
                tex_coords: [0.0, 1.0],
            },
            ImageVertex {
                position: [1.0, -1.0],
                tex_coords: [1.0, 0.0],
            },
            ImageVertex {
                position: [1.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
        ];
        let image_square_vertices = VertexBuffer::new(display, &image_square_shape).unwrap_or_else(|ex| {
            let msg = "Could not create image square vertices";
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
        let background_texture = Texture2d::new(display, background_image).unwrap_or_else(|ex| {
            let msg = "Could not create background texture";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
        let rect_program = Program::from_source(
            display,
            gl_utils::RECT_VERTEX_SHADER_SRC,
            gl_utils::RECT_FRAGMENT_SHADER_SRC,
            None,
        )
        .unwrap_or_else(|ex| {
            let msg = "Could not create OpenGL rect program";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
        let border_shape = vec![
            // left edge
            Vertex {
                position: [-1.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0 + BORDER_LINE_SCALE * 2.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0 + BORDER_LINE_SCALE * 2.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0 + BORDER_LINE_SCALE * 2.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            // top edge
            Vertex {
                position: [-1.0, 1.0 - BORDER_LINE_SCALE * 2.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, 1.0 - BORDER_LINE_SCALE * 2.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0 - BORDER_LINE_SCALE * 2.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            // right edge
            Vertex {
                position: [1.0 - BORDER_LINE_SCALE * 2.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0 - BORDER_LINE_SCALE * 2.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0 - BORDER_LINE_SCALE * 2.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            // bottom edge
            Vertex {
                position: [-1.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0 + BORDER_LINE_SCALE * 2.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, -1.0 + BORDER_LINE_SCALE * 2.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [-1.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, -1.0 + BORDER_LINE_SCALE * 2.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
            Vertex {
                position: [1.0, -1.0],
                color: [0.5, 0.5, 0.5, 1.0],
            },
        ];
        let border_vertices = VertexBuffer::new(display, &border_shape).unwrap_or_else(|ex| {
            let msg = "Could not create border vertices";
            error!("{}:\n{}", msg, ex);
            panic!("{}.", msg);
        });
        MlbGlUi {
            ui_info,
            image_program,
            image_square_vertices,
            background_texture,
            rect_program,
            border_vertices,
        }
    }

    /// Draws the MLB UI with the given parameters.
    ///
    /// # Errors
    /// Panics if the given target cannot be used to render the MLB UI.
    pub fn draw(&mut self, display: &Display, target: &mut Frame, text_brush_option: Option<&mut GlyphBrush>) {
        let screen_dims = display.get_framebuffer_dimensions();
        let screen_width = screen_dims.0 as f32;
        let screen_height = screen_dims.1 as f32;

        let background_uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0 , 0.0, 0.0, 1.0f32],
            ],
            tex: &self.background_texture,
        };
        target
            .draw(
                &self.image_square_vertices,
                &NoIndices(PrimitiveType::TriangleStrip),
                &self.image_program,
                &background_uniforms,
                &DrawParameters::default(),
            )
            .unwrap_or_else(|ex| {
                let msg = "Target could not draw background";
                error!("{}:\n{}", msg, ex);
                panic!("{}.", msg);
            });

        let focused_day = self.ui_info.focused_day;
        let focused_index = self.ui_info.focused_index;
        let (focused_translate_x, focused_translate_y) =
            calc_game_location_percentage(true, focused_index as f32, focused_day as f32);
        let x_offset = -1.0 + (focused_translate_x + BORDER_SCALE / 2.0) * 2.0 - BORDER_OFFSET;
        let y_offset = 1.0 - (focused_translate_y + BORDER_SCALE / 2.0) * 2.0 + BORDER_OFFSET;
        let border_uniforms = uniform! {
            matrix: [
                [BORDER_SCALE, 0.0, 0.0, 0.0],
                [0.0, BORDER_SCALE, 0.0, 0.0],
                [0.0, 0.0, BORDER_SCALE, 0.0],
                [x_offset, y_offset, 0.0, 1.0f32],
            ]
        };
        target
            .draw(
                &self.border_vertices,
                &NoIndices(PrimitiveType::TrianglesList),
                &self.rect_program,
                &border_uniforms,
                &DrawParameters::default(),
            )
            .unwrap_or_else(|ex| {
                let msg = "Target could not draw selected border";
                error!("{}:\n{}", msg, ex);
                panic!("{}.", msg);
            });

        for (row, day) in self.ui_info.days.iter_mut().enumerate() {
            for i in day.begin_index..(day.begin_index + X_PAGE_SIZE) {
                let col = i - day.begin_index;
                let game = &mut day.games[i];

                let x = col as f32;
                let y = row as f32;
                let (game_scale, translate_x, translate_y) = if row == focused_day && col == focused_index {
                    let game_scale = FOCUSED_GAME_SCALE;
                    let (translate_x, translate_y) = calc_game_location_percentage(true, x, y);
                    (game_scale, translate_x, translate_y)
                } else {
                    let game_scale = GAME_SCALE;
                    let (translate_x, translate_y) = calc_game_location_percentage(false, x, y);
                    (game_scale, translate_x, translate_y)
                };

                let x_offset = -1.0 + (translate_x + game_scale / 2.0) * 2.0;
                let y_offset = 1.0 - (translate_y + game_scale / 2.0) * 2.0;
                let game_uniforms = uniform! {
                    matrix: [
                        [game_scale, 0.0, 0.0, 0.0],
                        [0.0, game_scale, 0.0, 0.0],
                        [0.0, 0.0, game_scale, 0.0],
                        [x_offset, y_offset, 0.0, 1.0f32],
                    ],
                    tex: game.get_texture(&display),
                };
                target
                    .draw(
                        &self.image_square_vertices,
                        &NoIndices(PrimitiveType::TriangleStrip),
                        &self.image_program,
                        &game_uniforms,
                        &DrawParameters::default(),
                    )
                    .unwrap_or_else(|ex| {
                        let msg = "Target could not draw game";
                        error!("{}:\n{}", msg, ex);
                        panic!("{}.", msg);
                    });
            }
        }
        if let Some(text_brush) = text_brush_option {
            let focused_day_info = &self.ui_info.days[focused_day];
            let focused_game = &focused_day_info.games[focused_index + focused_day_info.begin_index].info;
            let x_offset = focused_translate_x * screen_width;
            let y_offset = (focused_translate_y - 0.05) * screen_height;
            let text_top_left = (x_offset, y_offset);
            text_brush.queue(Section {
                text: &focused_game.title,
                color: [1.0, 1.0, 1.0, 1.0f32],
                screen_position: text_top_left,
                ..Section::default()
            });
            let x_offset = focused_translate_x * screen_width;
            let y_offset = (focused_translate_y + FOCUSED_GAME_SCALE + 0.025) * screen_height;
            let text_top_left = (x_offset, y_offset);
            text_brush.queue(Section {
                text: &focused_game.summary,
                color: [1.0, 1.0, 1.0, 1.0f32],
                screen_position: text_top_left,
                ..Section::default()
            });
            text_brush.draw_queued(display, target);
        }
    }

    /// Moves the focus in the given direction. The visual representation will be updated on the next call to draw.
    pub fn move_focus(&mut self, direction: FocusDirection) {
        let info = &mut self.ui_info;
        let day = &mut info.days[info.focused_day];
        match direction {
            FocusDirection::Left => {
                if info.focused_index > 0 {
                    info.focused_index -= 1;
                } else if day.begin_index > 0 {
                    day.begin_index -= 1;
                }
            }
            FocusDirection::Right => {
                if info.focused_index < X_PAGE_SIZE - 1 {
                    info.focused_index += 1;
                } else if day.begin_index + X_PAGE_SIZE < day.games.len() {
                    day.begin_index += 1;
                }
            }
            FocusDirection::Up => {
                if info.focused_day > 0 {
                    info.focused_day -= 1;
                }
            }
            FocusDirection::Down => {
                if info.focused_day < info.days.len() - 1 {
                    info.focused_day += 1;
                }
            }
        }
    }
}

/// Calculates the percentage of the screen (assuming (0, 0) is the top-left corner) at which the top-left of the
/// game entry at the given indices should be rendered.
fn calc_game_location_percentage(focused: bool, x: f32, y: f32) -> (f32, f32) {
    if focused {
        let translate_x = LEFT_INDENT + (FOCUSED_GAME_SCALE * x) + (GAME_X_PADDING * x);
        let translate_y = TOP_INDENT + (FOCUSED_GAME_SCALE * y) + (GAME_Y_PADDING * 2.0 * y);
        (translate_x, translate_y)
    } else {
        let translate_x = LEFT_INDENT
            + (NON_FOCUSED_OFFSET * (x + 1.0))
            + (GAME_SCALE * x)
            + (NON_FOCUSED_OFFSET * x)
            + (GAME_X_PADDING * x);
        let translate_y = TOP_INDENT
            + (NON_FOCUSED_OFFSET * (y + 1.0))
            + (GAME_SCALE * y)
            + (NON_FOCUSED_OFFSET * y)
            + (GAME_Y_PADDING * 2.0 * y);
        (translate_x, translate_y)
    }
}

/// A container for backing information for a single game.
struct MlbGameGlInfo {
    info: MlbGameClientInfo,
    texture: Option<Texture2d>,
}

impl MlbGameGlInfo {
    /// Lazily initializes the texture for the game represented by this container.
    fn get_texture(&mut self, display: &Display) -> &Texture2d {
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

/// A container for backing information for a single day.
struct DayRowInfo {
    games: Vec<MlbGameGlInfo>,
    begin_index: usize,
}

impl DayRowInfo {
    pub fn new(games: Vec<MlbGameGlInfo>) -> Self {
        DayRowInfo { games, begin_index: 0 }
    }
}

/// A container for MLB UI backing information.
pub struct MlbUiInfo {
    days: Vec<DayRowInfo>,
    focused_day: usize,
    focused_index: usize,
}

impl MlbUiInfo {
    /// Asynchronously initializes the backing information container.
    pub async fn init() -> Self {
        let result = dss_mlb::get_games().await;
        let mut days = Vec::with_capacity(result.len());
        for day in result.values().rev() {
            let mut games: Vec<MlbGameGlInfo> = Vec::with_capacity(day.len());
            for game in day {
                games.push(game.to_owned().into());
            }
            days.push(DayRowInfo::new(games));
        }
        MlbUiInfo {
            days,
            focused_day: 0,
            focused_index: 0,
        }
    }
}
