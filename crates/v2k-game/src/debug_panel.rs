//! Secondary live diagnostics window for reverse-engineering work.

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::VideoSubsystem;
use std::time::{Duration, Instant};

const WIDTH: u32 = 600;
const HEIGHT: u32 = 640;
const SCALE: i32 = 2;
const LINE_HEIGHT: i32 = 18;

pub struct DebugPanel {
    canvas: Option<Canvas<Window>>,
    visible: bool,
    last_render: Option<Instant>,
    clipboard_text: String,
    copied_until: Option<Instant>,
}

impl Default for DebugPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugPanel {
    pub fn new() -> Self {
        Self {
            canvas: None,
            visible: false,
            last_render: None,
            clipboard_text: String::new(),
            copied_until: None,
        }
    }

    pub fn toggle(&mut self, video: &VideoSubsystem) -> Result<(), String> {
        if self.canvas.is_none() {
            let window = video
                .window("V2K Reverse Engineering Diagnostics", WIDTH, HEIGHT)
                .position(40, 40)
                .hidden()
                .build()
                .map_err(|error| error.to_string())?;
            let canvas = window
                .into_canvas()
                .software()
                .build()
                .map_err(|error| error.to_string())?;
            self.canvas = Some(canvas);
        }

        self.visible = !self.visible;
        self.last_render = None;
        if let Some(canvas) = &mut self.canvas {
            if self.visible {
                canvas.window_mut().show();
                canvas.window_mut().raise();
            } else {
                canvas.window_mut().hide();
            }
        }
        Ok(())
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn owns_window(&self, window_id: u32) -> bool {
        self.canvas
            .as_ref()
            .is_some_and(|canvas| canvas.window().id() == window_id)
    }

    pub fn hide(&mut self) {
        self.visible = false;
        if let Some(canvas) = &mut self.canvas {
            canvas.window_mut().hide();
        }
    }

    /// Copy the most recently rendered diagnostics when the panel's Copy
    /// button is clicked. SDL routes this through the platform clipboard.
    pub fn handle_click(&mut self, x: i32, y: i32, video: &VideoSubsystem) -> Result<bool, String> {
        if !self.visible || !copy_button_rect().contains_point((x, y)) {
            return Ok(false);
        }
        video.clipboard().set_clipboard_text(&self.clipboard_text)?;
        self.copied_until = Some(Instant::now() + Duration::from_secs(1));
        self.last_render = None;
        Ok(true)
    }

    pub fn render(&mut self, lines: &[String]) {
        if !self.visible {
            return;
        }
        let now = Instant::now();
        if self
            .last_render
            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(100))
        {
            return;
        }
        self.last_render = Some(now);
        self.clipboard_text = lines.join("\r\n");
        let copied = self.copied_until.is_some_and(|until| now < until);
        let Some(canvas) = &mut self.canvas else {
            return;
        };

        canvas.set_draw_color(Color::RGB(8, 12, 16));
        canvas.clear();
        let button = copy_button_rect();
        canvas.set_draw_color(if copied {
            Color::RGB(55, 115, 70)
        } else {
            Color::RGB(35, 48, 58)
        });
        let _ = canvas.fill_rect(button);
        canvas.set_draw_color(if copied {
            Color::RGB(145, 235, 165)
        } else {
            Color::RGB(255, 210, 64)
        });
        let _ = canvas.draw_rect(button);
        draw_text(
            canvas,
            if copied { 488 } else { 500 },
            14,
            if copied { "COPIED" } else { "COPY" },
            if copied {
                Color::RGB(145, 235, 165)
            } else {
                Color::RGB(255, 210, 64)
            },
        );
        for (index, line) in lines.iter().enumerate() {
            let y = 14 + index as i32 * LINE_HEIGHT;
            if y + LINE_HEIGHT >= HEIGHT as i32 {
                break;
            }
            let color = if line.starts_with('[') {
                Color::RGB(255, 210, 64)
            } else if line.starts_with("F12") {
                Color::RGB(120, 140, 150)
            } else {
                Color::RGB(145, 235, 165)
            };
            draw_text(canvas, 14, y, line, color);
        }
        canvas.present();
    }
}

fn copy_button_rect() -> Rect {
    Rect::new(476, 7, 110, 28)
}

fn draw_text(canvas: &mut Canvas<Window>, x: i32, y: i32, text: &str, color: Color) {
    canvas.set_draw_color(color);
    let mut pen_x = x;
    for character in text.to_ascii_uppercase().chars() {
        let glyph = glyph(character);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    let _ = canvas.fill_rect(Rect::new(
                        pen_x + column * SCALE,
                        y + row as i32 * SCALE,
                        SCALE as u32,
                        SCALE as u32,
                    ));
                }
            }
        }
        pen_x += 6 * SCALE;
    }
}

#[rustfmt::skip]
fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [14,17,17,31,17,17,17], 'B' => [30,17,17,30,17,17,30],
        'C' => [14,17,16,16,16,17,14], 'D' => [30,17,17,17,17,17,30],
        'E' => [31,16,16,30,16,16,31], 'F' => [31,16,16,30,16,16,16],
        'G' => [14,17,16,23,17,17,15], 'H' => [17,17,17,31,17,17,17],
        'I' => [14,4,4,4,4,4,14],       'J' => [7,2,2,2,18,18,12],
        'K' => [17,18,20,24,20,18,17],  'L' => [16,16,16,16,16,16,31],
        'M' => [17,27,21,21,17,17,17],  'N' => [17,25,21,19,17,17,17],
        'O' => [14,17,17,17,17,17,14],  'P' => [30,17,17,30,16,16,16],
        'Q' => [14,17,17,17,21,18,13],  'R' => [30,17,17,30,20,18,17],
        'S' => [15,16,16,14,1,1,30],    'T' => [31,4,4,4,4,4,4],
        'U' => [17,17,17,17,17,17,14],  'V' => [17,17,17,17,17,10,4],
        'W' => [17,17,17,21,21,21,10],  'X' => [17,17,10,4,10,17,17],
        'Y' => [17,17,10,4,4,4,4],      'Z' => [31,1,2,4,8,16,31],
        '0' => [14,17,19,21,25,17,14],  '1' => [4,12,4,4,4,4,14],
        '2' => [14,17,1,2,4,8,31],      '3' => [30,1,1,14,1,1,30],
        '4' => [2,6,10,18,31,2,2],      '5' => [31,16,16,30,1,1,30],
        '6' => [14,16,16,30,17,17,14],  '7' => [31,1,2,4,8,8,8],
        '8' => [14,17,17,14,17,17,14],  '9' => [14,17,17,15,1,1,14],
        ':' => [0,4,4,0,4,4,0],         '.' => [0,0,0,0,0,6,6],
        '-' => [0,0,0,31,0,0,0],        '+' => [0,4,4,31,4,4,0],
        '/' => [1,2,2,4,8,8,16],        '=' => [0,31,0,31,0,0,0],
        '%' => [17,2,4,8,17,0,0],       '(' => [2,4,8,8,8,4,2],
        ')' => [8,4,2,2,2,4,8],         '[' => [14,8,8,8,8,8,14],
        ']' => [14,2,2,2,2,2,14],       '_' => [0,0,0,0,0,0,31],
        ',' => [0,0,0,0,6,4,8],         '#' => [10,31,10,10,31,10,0],
        '?' => [14,17,1,2,4,0,4],       ' ' => [0; 7],
        _ => [31,17,2,4,0,4,0],
    }
}
