use ratatui::prelude::*;
use std::f64::consts::PI;

// --- EASING (Smooth S-Curve) ---
fn ease_in_out_cubic(x: f64) -> f64 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

// --- STATE ---
pub struct TomatoState {
    pub rotation: f64,
    pub wobble: f64,
    pub progress: f64,
    pub pause_timer: u16,
    pub is_spinning: bool,
    pub blink_timer: u16,
    pub is_blinking: bool,
    pub next_blink_in: u16,
}

impl Default for TomatoState {
    fn default() -> Self {
        Self::new()
    }
}

impl TomatoState {
    pub fn new() -> Self {
        Self {
            rotation: 0.0,
            wobble: 0.0,
            progress: 0.0,
            pause_timer: 40,
            is_spinning: false,
            blink_timer: 0,
            is_blinking: false,
            next_blink_in: 30,
        }
    }

    pub fn tick(&mut self) {
        // Blink
        if self.is_blinking {
            if self.blink_timer > 0 {
                self.blink_timer -= 1;
            } else {
                self.is_blinking = false;
                self.next_blink_in = (rand_simple(self.progress as u64) % 50 + 40) as u16;
            }
        } else if self.next_blink_in > 0 {
            self.next_blink_in -= 1;
        } else {
            self.is_blinking = true;
            self.blink_timer = 5;
        }

        // Spin
        if self.is_spinning {
            self.progress += 0.015;
            if self.progress >= 1.0 {
                self.progress = 0.0;
                self.rotation = 0.0;
                self.wobble = 0.0;
                self.is_spinning = false;
                self.pause_timer = 50;
            } else {
                let eased = ease_in_out_cubic(self.progress);
                self.rotation = eased * 2.0 * PI;
                // Wobble head sideways
                self.wobble = (self.progress * PI).sin() * 0.5;
            }
        } else if self.pause_timer > 0 {
            self.pause_timer -= 1;
        } else {
            self.is_spinning = true;
        }
    }
}

fn rand_simple(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}

struct Point {
    x: f64,
    y: f64,
    z: f64,
}

struct Normal {
    nx: f64,
    ny: f64,
    nz: f64,
}

struct Context {
    cx: f64,
    cy: f64,
    nlx: f64,
    nly: f64,
    nlz: f64,
    width: usize,
    height: usize,
}

struct Buffers<'a> {
    z_buffer: &'a mut [f64],
    char_buffer: &'a mut [char],
    color_buffer: &'a mut [Color],
}

// --- WIDGET ---
pub struct Tomato;

impl StatefulWidget for Tomato {
    type State = TomatoState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let width = area.width as usize;
        let height = area.height as usize;

        // If area is too small, don't render to avoid crashes or weird artifacts
        if width < 5 || height < 5 {
            return;
        }

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        let spin_ang = state.rotation;
        // Rest wobble: -0.15 makes it look cute/shy when stopped
        let wobble_ang = state.wobble - 0.15;

        // FIXED: Negative tilt tips the Top (+Y) TOWARDS the viewer (-Z)
        // This ensures the leaves are visible on top
        let camera_tilt: f64 = -0.55;

        let mut z_buffer = vec![f64::NEG_INFINITY; width * height];
        let mut char_buffer = vec![' '; width * height];
        let mut color_buffer = vec![Color::Reset; width * height];

        // Light: From Top-Left-Front
        let lx: f64 = -0.5;
        let ly: f64 = 0.8; // High light to catch the leaves
        let lz: f64 = -1.0;
        let l_len = (lx * lx + ly * ly + lz * lz).sqrt();
        let (nlx, nly, nlz) = (lx / l_len, ly / l_len, lz / l_len);

        let ctx = Context {
            cx,
            cy,
            nlx,
            nly,
            nlz,
            width,
            height,
        };

        let radius_x = 13.0;
        let radius_y = 10.5;

        // --- SPHERE ---
        let mut theta = 0.0_f64;
        while theta < 2.0 * PI {
            let mut phi = 0.0_f64;
            while phi < PI {
                let sinphi = phi.sin();
                let cosphi = phi.cos();
                let costheta = theta.cos();
                let sintheta = theta.sin();

                // Geometry (Y is Up)
                let x = radius_x * sinphi * costheta;
                let y = radius_y * cosphi;
                let z = radius_x * sinphi * sintheta;

                let display_char;
                let mut display_color = Color::Red;
                let mut is_bright = false;

                // 1. LEAVES
                // phi starts at 0 (Top). We want the top ~0.6 radians to be green
                // We add theta noise to make the star shape
                let leaf_threshold = 0.6 + 0.15 * (5.0 * theta).cos();
                if phi < leaf_threshold {
                    display_char = '%';
                    display_color = Color::Green;
                    // Make leaves slightly brighter/rougher
                    is_bright = true;
                } else {
                    // 2. FACE
                    // Front is -Z, which corresponds to theta = 3*PI/2
                    let face_theta = 3.0 * PI / 2.0;

                    let theta_diff = (theta - face_theta).abs();

                    // Eyes: slightly above equator (phi 1.3)
                    let eye_phi = 1.3;
                    let eye_spacing = 0.35;

                    let left_eye = (theta - (face_theta - eye_spacing)).abs() < 0.15
                        && (phi - eye_phi).abs() < 0.15;
                    let right_eye = (theta - (face_theta + eye_spacing)).abs() < 0.15
                        && (phi - eye_phi).abs() < 0.15;

                    // Blush: below eyes
                    let blush = theta_diff < 0.7 && (phi - 1.8).abs() < 0.1;

                    if left_eye || right_eye {
                        if state.is_blinking {
                            display_char = '-';
                            is_bright = true;
                        } else {
                            display_char = 'O';
                            display_color = Color::White;
                            is_bright = true;
                        }
                    } else if blush {
                        display_char = '=';
                        display_color = Color::LightRed;
                    } else {
                        // Skin
                        display_char = '?';
                    }
                }

                // Normals
                let nx = sinphi * costheta;
                let ny = cosphi;
                let nz = sinphi * sintheta;

                // Rotations
                // 1. Spin (Y)
                let (sa, ca) = (spin_ang.sin(), spin_ang.cos());
                let x_a = x * ca - z * sa;
                let y_a = y;
                let z_a = x * sa + z * ca;
                let nx_a = nx * ca - nz * sa;
                let ny_a = ny;
                let nz_a = nx * sa + nz * ca;

                // 2. Wobble (Z)
                let (sb, cb) = (wobble_ang.sin(), wobble_ang.cos());
                let x_b = x_a * cb - y_a * sb;
                let y_b = x_a * sb + y_a * cb;
                let z_b = z_a;
                let nx_b = nx_a * cb - ny_a * sb;
                let ny_b = nx_a * sb + ny_a * cb;
                let nz_b = nz_a;

                // 3. Camera (X)
                let (sc, cc) = (camera_tilt.sin(), camera_tilt.cos());
                let x_final = x_b;
                let y_final = y_b * cc - z_b * sc;
                let z_final = y_b * sc + z_b * cc;
                let nx_final = nx_b;
                let ny_final = ny_b * cc - nz_b * sc;
                let nz_final = ny_b * sc + nz_b * cc;

                let mut bufs = Buffers {
                    z_buffer: &mut z_buffer,
                    char_buffer: &mut char_buffer,
                    color_buffer: &mut color_buffer,
                };

                plot_point_final(
                    Point {
                        x: x_final,
                        y: y_final,
                        z: z_final,
                    },
                    Normal {
                        nx: nx_final,
                        ny: ny_final,
                        nz: nz_final,
                    },
                    &ctx,
                    &mut bufs,
                    display_char,
                    display_color,
                    is_bright,
                );

                phi += 0.04;
            }
            theta += 0.04;
        }

        // --- STEM ---
        let stem_height = 2.5;
        let stem_radius = 0.6;
        let mut h = 0.0;
        while h < stem_height {
            let mut ang = 0.0;
            while ang < 2.0 * PI {
                let x = stem_radius * ang.cos();
                let z = stem_radius * ang.sin();
                let y = radius_y + h;

                // 1. Spin
                let (sa, ca) = (spin_ang.sin(), spin_ang.cos());
                let x_a = x * ca - z * sa;
                let y_a = y;
                let z_a = x * sa + z * ca;

                // 2. Wobble
                let (sb, cb) = (wobble_ang.sin(), wobble_ang.cos());
                let x_b = x_a * cb - y_a * sb;
                let y_b = x_a * sb + y_a * cb;
                let z_b = z_a;

                // 3. Camera
                let (sc, cc) = (camera_tilt.sin(), camera_tilt.cos());
                let x_final = x_b;
                let y_final = y_b * cc - z_b * sc;
                let z_final = y_b * sc + z_b * cc;

                let mut bufs = Buffers {
                    z_buffer: &mut z_buffer,
                    char_buffer: &mut char_buffer,
                    color_buffer: &mut color_buffer,
                };

                plot_point_final(
                    Point {
                        x: x_final,
                        y: y_final,
                        z: z_final,
                    },
                    Normal {
                        nx: 0.0,
                        ny: 1.0,
                        nz: 0.0,
                    },
                    &ctx,
                    &mut bufs,
                    '|',
                    Color::Green,
                    false,
                );
                ang += 0.4;
            }
            h += 0.2;
        }

        // Render
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if char_buffer[idx] != ' ' {
                    let gx = area.left() + x as u16;
                    let gy = area.top() + y as u16;
                    if gx < buf.area.width && gy < buf.area.height {
                        let cell = &mut buf[(gx, gy)];
                        cell.set_char(char_buffer[idx]).set_fg(color_buffer[idx]);
                    }
                }
            }
        }
    }
}

fn plot_point_final(
    p: Point,
    n: Normal,
    ctx: &Context,
    bufs: &mut Buffers,
    forced_char: char,
    forced_color: Color,
    is_bright: bool,
) {
    let luminance = n.nx * ctx.nlx + n.ny * ctx.nly + n.nz * ctx.nlz;

    if luminance > -0.2 || is_bright {
        let k2 = 45.0;
        let ooz = 1.0 / (p.z + k2);

        // NOTE: Minus Y because screen coordinates go down
        let xp = (ctx.cx + 45.0 * ooz * p.x) as usize;
        let yp = (ctx.cy - 45.0 * ooz * p.y * 0.55) as usize;

        if xp < ctx.width && yp < ctx.height {
            let idx = yp * ctx.width + xp;
            if ooz > bufs.z_buffer[idx] {
                bufs.z_buffer[idx] = ooz;

                if forced_char != '?' {
                    bufs.char_buffer[idx] = forced_char;
                    bufs.color_buffer[idx] = forced_color;
                } else {
                    let skin_chars = ".,-~:;=!*#$@";
                    let l_idx = (luminance.max(0.0) * (skin_chars.len() as f64 - 1.0)) as usize;
                    bufs.char_buffer[idx] = skin_chars.chars().nth(l_idx).unwrap_or('@');
                    bufs.color_buffer[idx] = forced_color;
                }
            }
        }
    }
}
