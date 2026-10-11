use anyhow::Result;
use sdl3::{
    pixels::Color,
    rect::Rect,
    render::{BlendMode, Canvas, RenderTarget},
};

pub const LABELS: [&str; 12] = [
    "B", "Y", "Select", "Start", "Up", "Down", "Left", "Right", "A", "X", "L", "R",
];
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum View {
    Main,
    Keys,
    Save,
    Load,
    Shaders,
}
#[derive(Clone, Copy)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
}
pub struct Menu {
    pub open: bool,
    pub view: View,
    pub selected: usize,
    pub controller: bool,
    pub capturing: bool,
    pub message: String,
}
impl Default for Menu {
    fn default() -> Self {
        Self {
            open: false,
            view: View::Main,
            selected: 0,
            controller: false,
            capturing: false,
            message: String::new(),
        }
    }
}
impl Menu {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.view = View::Main;
        self.selected = 0;
        self.capturing = false;
        self.message.clear();
    }
    pub fn navigate(&mut self, action: Action) {
        let count = if self.view == View::Keys {
            12
        } else if self.view == View::Shaders {
            crate::gpu::Shader::ALL.len()
        } else if self.view == View::Main {
            6
        } else {
            5
        };
        match action {
            Action::Up => self.selected = (self.selected + count - 1) % count,
            Action::Down => self.selected = (self.selected + 1) % count,
            Action::Left | Action::Right if self.view == View::Keys => {
                self.controller = !self.controller
            }
            Action::Back => {
                if self.capturing {
                    self.capturing = false;
                } else if self.view != View::Main {
                    self.view = View::Main;
                    self.selected = 0;
                } else {
                    self.open = false;
                }
            }
            _ => {}
        }
        self.message.clear();
    }
    pub fn draw<T: RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        rows: &[String],
        footer: &str,
        status: &str,
    ) -> Result<()> {
        let (w, h) = canvas.output_size()?;
        canvas.set_blend_mode(BlendMode::Blend);
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 175));
        canvas.fill_rect(None)?;
        // SDL's built-in 8px bitmap font: integer scaling keeps the glyphs crisp.
        let scale = ((w / 480).min(h / 340)).clamp(1, 4) as f32;
        canvas.set_scale(scale, scale)?;
        let lw = w as f32 / scale;
        let lh = h as f32 / scale;
        let panel_w = 460u32.min(lw as u32);
        let panel_h = if self.view == View::Keys {
            324
        } else if self.view == View::Main {
            242
        } else {
            218
        };
        let x = ((lw - panel_w as f32) / 2.0).max(0.0) as i32;
        let y = ((lh - panel_h as f32) / 2.0).max(0.0) as i32;
        canvas.set_draw_color(Color::RGB(17, 22, 35));
        canvas.fill_rect(Rect::new(x, y, panel_w, panel_h))?;
        canvas.set_draw_color(Color::RGB(112, 210, 190));
        canvas.draw_rect(Rect::new(x, y, panel_w, panel_h))?;
        let title = match self.view {
            View::Main => "PAUSED",
            View::Keys => {
                if self.controller {
                    "KEYBINDS / CONTROLLER"
                } else {
                    "KEYBINDS / KEYBOARD"
                }
            }
            View::Save => "SAVE STATE",
            View::Load => "LOAD STATE",
            View::Shaders => "SCANLINES / CRT SHADER",
        };
        canvas.draw_debug_text(title, (x as f32 + 20.0, y as f32 + 18.0))?;
        for (i, row) in rows.iter().enumerate() {
            let ry = y + 44 + i as i32 * 18;
            if i == self.selected {
                canvas.set_draw_color(Color::RGB(38, 66, 76));
                canvas.fill_rect(Rect::new(x + 12, ry - 5, panel_w.saturating_sub(24), 18))?;
                canvas.set_draw_color(Color::RGB(160, 245, 212));
            } else {
                canvas.set_draw_color(Color::RGB(222, 229, 240));
            }
            let text = format!(
                "{} {}",
                if i == self.selected { ">" } else { " " },
                ascii(row, 52)
            );
            canvas.draw_debug_text(&text, ((x + 20) as f32, ry as f32))?;
        }
        canvas.set_draw_color(Color::RGB(168, 180, 201));
        canvas.draw_debug_text(
            &ascii(footer, 52),
            ((x + 20) as f32, (y + panel_h as i32 - 50) as f32),
        )?;
        canvas.set_draw_color(Color::RGB(160, 245, 212));
        canvas.draw_debug_text(
            &ascii(
                if self.message.is_empty() {
                    status
                } else {
                    &self.message
                },
                52,
            ),
            ((x + 20) as f32, (y + panel_h as i32 - 32) as f32),
        )?;
        canvas.set_draw_color(Color::RGB(168, 180, 201));
        canvas.draw_debug_text(
            "F1 or Start+Select: resume   Esc: exit",
            ((x + 20) as f32, (y + panel_h as i32 - 16) as f32),
        )?;
        canvas.set_scale(1.0, 1.0)?;
        Ok(())
    }
}
fn ascii(value: &str, limit: usize) -> String {
    value
        .chars()
        .take(limit)
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exit_is_reachable_without_adding_save_slots() {
        let mut menu = Menu::default();
        menu.navigate(Action::Up);
        assert_eq!(menu.selected, 5);
        menu.navigate(Action::Down);
        assert_eq!(menu.selected, 0);
        menu.view = View::Save;
        menu.navigate(Action::Up);
        assert_eq!(menu.selected, 4);
    }
}
