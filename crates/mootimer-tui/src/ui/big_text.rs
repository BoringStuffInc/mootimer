use ratatui::prelude::*;
use ratatui::widgets::Widget;

pub struct BigText<'a> {
    text: &'a str,
    style: Style,
}

impl<'a> BigText<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for BigText<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut x_offset = 0;
        let char_spacing = 1;

        for ch in self.text.chars() {
            let (width, lines) = get_char_bitmap(ch);

            if x_offset + width as u16 > area.width {
                break;
            }

            for (y, line) in lines.iter().enumerate() {
                if y as u16 >= area.height {
                    break;
                }

                for (x, active) in line.iter().enumerate() {
                    if *active {
                        let gx = area.left() + x_offset + x as u16;
                        let gy = area.top() + y as u16;

                        if gx < buf.area.width && gy < buf.area.height {
                            if let Some(cell) = buf.cell_mut((gx, gy)) {
                                cell.set_style(self.style).set_symbol("█");
                            }
                        }
                    }
                }
            }
            x_offset += width as u16 + char_spacing;
        }
    }
}

fn get_char_bitmap(ch: char) -> (usize, Vec<Vec<bool>>) {
    match ch {
        '0' => (3, vec![
            vec![true, true, true],
            vec![true, false, true],
            vec![true, false, true],
            vec![true, false, true],
            vec![true, true, true],
        ]),
        '1' => (3, vec![
            vec![false, true, false],
            vec![true, true, false],
            vec![false, true, false],
            vec![false, true, false],
            vec![true, true, true],
        ]),
        '2' => (3, vec![
            vec![true, true, true],
            vec![false, false, true],
            vec![true, true, true],
            vec![true, false, false],
            vec![true, true, true],
        ]),
        '3' => (3, vec![
            vec![true, true, true],
            vec![false, false, true],
            vec![true, true, true],
            vec![false, false, true],
            vec![true, true, true],
        ]),
        '4' => (3, vec![
            vec![true, false, true],
            vec![true, false, true],
            vec![true, true, true],
            vec![false, false, true],
            vec![false, false, true],
        ]),
        '5' => (3, vec![
            vec![true, true, true],
            vec![true, false, false],
            vec![true, true, true],
            vec![false, false, true],
            vec![true, true, true],
        ]),
        '6' => (3, vec![
            vec![true, true, true],
            vec![true, false, false],
            vec![true, true, true],
            vec![true, false, true],
            vec![true, true, true],
        ]),
        '7' => (3, vec![
            vec![true, true, true],
            vec![false, false, true],
            vec![false, false, true],
            vec![false, false, true],
            vec![false, false, true],
        ]),
        '8' => (3, vec![
            vec![true, true, true],
            vec![true, false, true],
            vec![true, true, true],
            vec![true, false, true],
            vec![true, true, true],
        ]),
        '9' => (3, vec![
            vec![true, true, true],
            vec![true, false, true],
            vec![true, true, true],
            vec![false, false, true],
            vec![true, true, true],
        ]),
        ':' => (1, vec![
            vec![false],
            vec![true],
            vec![false],
            vec![true],
            vec![false],
        ]),
        _ => (3, vec![
            vec![false, false, false],
            vec![false, false, false],
            vec![false, false, false],
            vec![false, false, false],
            vec![false, false, false],
        ]),
    }
}
