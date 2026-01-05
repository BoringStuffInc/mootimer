use ratatui::prelude::*;

fn is_in_ellipse(px: f64, py: f64, cx: f64, cy: f64, rx: f64, ry: f64, angle: f64) -> bool {
    let dx = px - cx;
    let dy = py - cy;
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let local_x = dx * cos_a + dy * sin_a;
    let local_y = dy * cos_a - dx * sin_a;

    (local_x * local_x) / (rx * rx) + (local_y * local_y) / (ry * ry) <= 1.0
}

fn rand_pseudo(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}

pub struct CowState {
    pub time: f64,
    pub blink_timer: u16,
    pub next_blink: u16,
    pub is_blinking: bool,
    pub chew_phase: f64,
}

impl Default for CowState {
    fn default() -> Self {
        Self::new()
    }
}

impl CowState {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            blink_timer: 0,
            next_blink: 30,
            is_blinking: false,
            chew_phase: 0.0,
        }
    }

    pub fn tick(&mut self) {
        self.time += 0.05;
        self.chew_phase += 0.15;

        if self.is_blinking {
            if self.blink_timer > 0 {
                self.blink_timer -= 1;
            } else {
                self.is_blinking = false;
                self.next_blink = (rand_pseudo(self.time as u64) % 60 + 30) as u16;
            }
        } else if self.next_blink > 0 {
            self.next_blink -= 1;
        } else {
            self.is_blinking = true;
            self.blink_timer = 15;
        }
    }
}

pub struct Cow;

impl StatefulWidget for Cow {
    type State = CowState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let width = area.width as usize;
        let height = area.height as usize;

        if width < 5 || height < 5 {
            return;
        }

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        let chew_y = (state.chew_phase.sin().abs()) * 1.5;
        let chew_x = state.chew_phase.cos() * 0.8;

        let ear_wiggle = (state.time * 0.5).sin() * 0.1;

        let color_horn = Color::DarkGray;
        let color_white = Color::White;
        let color_snout = Color::Rgb(255, 150, 150);
        let color_brown = Color::Rgb(110, 70, 30);
        let color_grass = Color::Green;

        for y in 0..height {
            for x in 0..width {
                let u = x as f64 - cx;
                let v = (y as f64 - cy) * 2.0;

                let mut char_to_draw = ' ';
                let mut color_to_draw = Color::Reset;
                let mut layer = 0;

                if is_in_ellipse(u, v, -9.0, -14.0, 2.5, 6.0, -0.5) {
                    char_to_draw = '(';
                    color_to_draw = color_horn;
                    layer = 1;
                }
                if is_in_ellipse(u, v, 9.0, -14.0, 2.5, 6.0, 0.5) {
                    char_to_draw = ')';
                    color_to_draw = color_horn;
                    layer = 1;
                }

                if is_in_ellipse(
                    u,
                    v,
                    -14.0,
                    -4.0 + ear_wiggle * 5.0,
                    6.0,
                    3.0,
                    -0.2 + ear_wiggle,
                ) {
                    char_to_draw = '~';
                    color_to_draw = color_white;
                    layer = 2;
                }
                if is_in_ellipse(
                    u,
                    v,
                    14.0,
                    -4.0 - ear_wiggle * 5.0,
                    6.0,
                    3.0,
                    0.2 - ear_wiggle,
                ) {
                    char_to_draw = '~';
                    color_to_draw = color_white;
                    layer = 2;
                }

                if is_in_ellipse(u, v, 0.0, -5.0, 11.0, 10.0, 0.0) {
                    let tex = if (u + v * 3.0) as i32 % 5 == 0 {
                        '"'
                    } else {
                        '#'
                    };
                    char_to_draw = tex;
                    color_to_draw = color_white;
                    layer = 3;
                }

                let snout_x = chew_x;
                let snout_y = 6.0 + chew_y;

                if is_in_ellipse(u, v, snout_x, snout_y, 12.0, 7.0, 0.0) {
                    char_to_draw = '@';
                    color_to_draw = color_snout;
                    layer = 4;
                }

                if is_in_ellipse(
                    u,
                    v,
                    snout_x - 4.0,
                    snout_y + 1.0 + chew_y * 0.5,
                    1.5,
                    1.0,
                    0.0,
                ) {
                    char_to_draw = 'O';
                    color_to_draw = color_brown;
                    layer = 5;
                }
                if is_in_ellipse(
                    u,
                    v,
                    snout_x + 4.0,
                    snout_y + 1.0 + chew_y * 0.5,
                    1.5,
                    1.0,
                    0.0,
                ) {
                    char_to_draw = 'O';
                    color_to_draw = color_brown;
                    layer = 5;
                }

                let grass_origin_x = snout_x + 2.0;
                let grass_origin_y = snout_y + 5.5;
                let grass_angle = 0.3 + (state.chew_phase.sin() * 0.2);

                let dx_grass = u - grass_origin_x;
                let dy_grass = v - grass_origin_y;

                if (0.0..14.0).contains(&dx_grass) {
                    let ideal_y = dx_grass * grass_angle.tan();
                    if (dy_grass - ideal_y).abs() < 1.5 {
                        char_to_draw = '\\';
                        color_to_draw = color_grass;
                        layer = 6;
                    }
                }

                let eye_y = -6.0;
                let eye_offset_x = 6.0;

                if is_in_ellipse(u, v, -eye_offset_x, eye_y, 1.8, 1.8, 0.0) {
                    layer = 7;
                    if state.is_blinking {
                        char_to_draw = '-';
                        color_to_draw = color_brown;
                    } else {
                        char_to_draw = 'O';
                        color_to_draw = color_brown;
                    }
                }
                if is_in_ellipse(u, v, eye_offset_x, eye_y, 1.8, 1.8, 0.0) {
                    layer = 7;
                    if state.is_blinking {
                        char_to_draw = '-';
                        color_to_draw = color_brown;
                    } else {
                        char_to_draw = 'O';
                        color_to_draw = color_brown;
                    }
                }

                if layer > 0 {
                    let gx = area.left() + x as u16;
                    let gy = area.top() + y as u16;
                    if gx < buf.area.width
                        && gy < buf.area.height
                        && let Some(cell) = buf.cell_mut((gx, gy))
                    {
                        cell.set_char(char_to_draw).set_fg(color_to_draw);
                    }
                }
            }
        }
    }
}
