use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

pub const SSD1306_WIDTH: usize = 128;
pub const SSD1306_HEIGHT: usize = 64;
pub const SSD1306_PAGE_SIZE: usize = 8;
pub const SSD1306_PAGES: usize = SSD1306_HEIGHT / SSD1306_PAGE_SIZE;

const FONT: [[u8; 5]; 55] = [
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // 0
    [0x00, 0x42, 0x7F, 0x40, 0x00], // 1
    [0x42, 0x61, 0x51, 0x49, 0x46], // 2
    [0x21, 0x41, 0x45, 0x4B, 0x31], // 3
    [0x18, 0x14, 0x12, 0x7F, 0x10], // 4
    [0x27, 0x45, 0x45, 0x45, 0x39], // 5
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // 6
    [0x01, 0x71, 0x09, 0x05, 0x03], // 7
    [0x36, 0x49, 0x49, 0x49, 0x36], // 8
    [0x06, 0x49, 0x49, 0x29, 0x1E], // 9
    [0x08, 0x08, 0x08, 0x08, 0x08], // 10 = '-'
    [0x00, 0x00, 0x00, 0x00, 0x00], // 11 = ' '
    // A–Z (index 12–37)
    [0x7E, 0x11, 0x11, 0x11, 0x7E], // 12 = 'A'
    [0x7F, 0x49, 0x49, 0x49, 0x36], // 13 = 'B'
    [0x3E, 0x41, 0x41, 0x41, 0x22], // 14 = 'C'
    [0x7F, 0x41, 0x41, 0x22, 0x1C], // 15 = 'D'
    [0x7F, 0x49, 0x49, 0x49, 0x41], // 16 = 'E'
    [0x7F, 0x09, 0x09, 0x09, 0x01], // 17 = 'F'
    [0x3E, 0x41, 0x49, 0x49, 0x7A], // 18 = 'G'
    [0x7F, 0x08, 0x08, 0x08, 0x7F], // 19 = 'H'
    [0x00, 0x41, 0x7F, 0x41, 0x00], // 20 = 'I'
    [0x20, 0x40, 0x41, 0x3F, 0x01], // 21 = 'J'
    [0x7F, 0x08, 0x14, 0x22, 0x41], // 22 = 'K'
    [0x7F, 0x40, 0x40, 0x40, 0x40], // 23 = 'L'
    [0x7F, 0x02, 0x0C, 0x02, 0x7F], // 24 = 'M'
    [0x7F, 0x04, 0x08, 0x10, 0x7F], // 25 = 'N'
    [0x3E, 0x41, 0x41, 0x41, 0x3E], // 26 = 'O'
    [0x7F, 0x09, 0x09, 0x09, 0x06], // 27 = 'P'
    [0x3E, 0x41, 0x51, 0x21, 0x5E], // 28 = 'Q'
    [0x7F, 0x09, 0x19, 0x29, 0x46], // 29 = 'R'
    [0x46, 0x49, 0x49, 0x49, 0x31], // 30 = 'S'
    [0x01, 0x01, 0x7F, 0x01, 0x01], // 31 = 'T'
    [0x3F, 0x40, 0x40, 0x40, 0x3F], // 32 = 'U'
    [0x1F, 0x20, 0x40, 0x20, 0x1F], // 33 = 'V'
    [0x3F, 0x40, 0x38, 0x40, 0x3F], // 34 = 'W'
    [0x63, 0x14, 0x08, 0x14, 0x63], // 35 = 'X'
    [0x07, 0x08, 0x70, 0x08, 0x07], // 36 = 'Y'
    [0x61, 0x51, 0x49, 0x45, 0x43], // 37 = 'Z'
    [0x00, 0x00, 0x60, 0x60, 0x00], // 38 = '.'
    [0x00, 0x3E, 0x41, 0x41, 0x00], // 39 = '('
    [0x00, 0x41, 0x41, 0x3E, 0x00], // 40 = ')'
    [0x00, 0x40, 0x50, 0x30, 0x00], // 41 = ','
    [0x00, 0x7F, 0x41, 0x41, 0x00], // 42 = '['
    [0x00, 0x41, 0x41, 0x7F, 0x00], // 43 = ']'
    [0x23, 0x13, 0x08, 0x64, 0x62], // 44 = '%'
    [0x08, 0x14, 0x22, 0x41, 0x00], // 45 = '<'
    [0x00, 0x41, 0x22, 0x14, 0x08], // 46 = '>'
    [0x00, 0x24, 0x24, 0x24, 0x00], // 47 = '='
    [0x02, 0x01, 0x51, 0x09, 0x06], // 48 = '?'
    [0x00, 0x00, 0x5F, 0x00, 0x00], // 49 = '!'
    [0x00, 0x36, 0x36, 0x00, 0x00], // 50 = ':'
    [0x08, 0x08, 0x3E, 0x08, 0x08], // 51 = '+'
    [0x20, 0x10, 0x08, 0x04, 0x02], // 52 = '/'
    [0x00, 0x00, 0x7F, 0x00, 0x00], // 53 = '|'
    [0x40, 0x40, 0x40, 0x40, 0x40], // 54 = '_'
];

#[derive(Debug)]
pub struct SSD1306<Driver: I2c> {
    i2c: Driver,
    addr: u8,
    frame_buf: [u8; SSD1306_WIDTH * SSD1306_PAGES],
}

impl<Driver: I2c> SSD1306<Driver> {
    pub fn new(i2c: Driver, addr: u8) -> Self {
        Self {
            i2c,
            addr,
            frame_buf: [0u8; SSD1306_WIDTH * SSD1306_PAGES],
        }
    }

    pub async fn init(&mut self) -> Result<(), Driver::Error> {
        Timer::after_millis(200).await;

        self.send_command(&[0xAE]).await?; // Display OFF
        self.send_command(&[0xD5, 0x80]).await?; // Osc freq
        self.send_command(&[0xA8, 0x3F]).await?; // Multiplex 64
        self.send_command(&[0xD3, 0x00]).await?; // Display offset 0
        self.send_command(&[0x40]).await?; // Start line 0
        self.send_command(&[0x8D, 0x14]).await?; // Charge pump ON
        self.send_command(&[0x20, 0x00]).await?; // Horizontal addressing mode
        self.send_command(&[0xA1]).await?; // Segment remap
        self.send_command(&[0xC8]).await?; // COM scan dec
        self.send_command(&[0xDA, 0x12]).await?; // COM pins
        self.send_command(&[0x81, 0xCF]).await?; // Contrast
        self.send_command(&[0xD9, 0xF1]).await?; // Pre-charge
        self.send_command(&[0xDB, 0x40]).await?; // VCOMH
        self.send_command(&[0xA4]).await?; // RAM display
        self.send_command(&[0xA6]).await?; // Normal
        self.send_command(&[0xAF]).await?;

        self.clear();
        self.flush().await
    }

    async fn send_command(&mut self, cmds: &[u8]) -> Result<(), Driver::Error> {
        let mut buf = [0x00; 8];

        buf[1..cmds.len() + 1].copy_from_slice(cmds);

        self.i2c.write(self.addr, &buf[..cmds.len() + 1]).await
    }

    pub fn clear(&mut self) {
        self.frame_buf.fill(0x00);
    }

    pub fn fill(&mut self) {
        self.frame_buf.fill(0xFF);
    }

    pub fn draw_pixel(&mut self, x: u8, y: u8, on: bool) {
        if x >= SSD1306_WIDTH as u8 || y >= SSD1306_HEIGHT as u8 {
            return;
        }
        let page = (y / 8) as usize;
        let bit = y % 8;
        let idx = page * SSD1306_WIDTH + x as usize;
        if on {
            self.frame_buf[idx] |= 1 << bit;
        } else {
            self.frame_buf[idx] &= !(1 << bit);
        }
    }

    pub fn draw_hline(&mut self, x: u8, y: u8, w: u8, on: bool) {
        for i in 0..w {
            self.draw_pixel(x + i, y, on);
        }
    }

    pub fn draw_vline(&mut self, x: u8, y: u8, h: u8, on: bool) {
        for i in 0..h {
            self.draw_pixel(x, y + i, on);
        }
    }

    pub fn draw_rect(&mut self, x: u8, y: u8, w: u8, h: u8, on: bool) {
        self.draw_hline(x, y, w, on);
        self.draw_hline(x, y + h - 1, w, on);
        self.draw_vline(x, y, h, on);
        self.draw_vline(x + w - 1, y, h, on);
    }

    pub fn draw_filled_rect(&mut self, x: u8, y: u8, w: u8, h: u8, on: bool) {
        for row in 0..h {
            self.draw_hline(x, y + row, w, on);
        }
    }

    fn char_to_glyph(c: u8) -> Option<usize> {
        match c {
            b'0'..=b'9' => Some((c - b'0') as usize),
            b'-' => Some(10),
            b' ' => Some(11),
            b'A'..=b'Z' => Some((c - b'A') as usize + 12),
            b'a'..=b'z' => Some((c - b'a') as usize + 12),
            b'.' => Some(38),
            b'(' => Some(39),
            b')' => Some(40),
            b',' => Some(41),
            b'[' => Some(42),
            b']' => Some(43),
            b'%' => Some(44),
            b'<' => Some(45),
            b'>' => Some(46),
            b'=' => Some(47),
            b'?' => Some(48),
            b'!' => Some(49),
            b':' => Some(50),
            b'+' => Some(51),
            b'/' => Some(52),
            b'|' => Some(53),
            b'_' => Some(54),
            _ => None,
        }
    }

    pub fn draw_char(&mut self, x: u8, page: u8, glyph_idx: usize) {
        for col in 0..5usize {
            let byte = FONT[glyph_idx][col];
            let fb_idx = page as usize * SSD1306_WIDTH + x as usize + col;
            if fb_idx < self.frame_buf.len() {
                self.frame_buf[fb_idx] = byte;
            }
        }
    }

    pub fn draw_bitmap(&mut self, x: u8, y: u8, w: u8, h: u8, data: &[u8]) {
        let stride = (w as usize + 7) / 8;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let byte_idx = row * stride + col / 8;
                let bit = 7 - (col % 8);
                let on = byte_idx < data.len() && (data[byte_idx] >> bit) & 1 == 1;
                self.draw_pixel(x + col as u8, y + row as u8, on);
            }
        }
    }

    pub fn draw_i16(&mut self, mut x: u8, page: u8, val: i16) -> u8 {
        if val < 0 {
            self.draw_char(x, page, 10); // '-'
            x += 6;
        }

        let mut n = val.unsigned_abs();
        let mut digits = [0u8; 5];
        let mut count = 0;

        loop {
            digits[count] = (n % 10) as u8;
            n /= 10;
            count += 1;
            if n == 0 {
                break;
            }
        }

        for i in (0..count).rev() {
            self.draw_char(x, page, digits[i] as usize);
            x += 6;
        }
        x
    }

    pub fn draw_str(&mut self, mut x: u8, page: u8, text: &[u8]) -> u8 {
        for &c in text {
            if let Some(idx) = Self::char_to_glyph(c) {
                self.draw_char(x, page, idx);
            }
            x = x.saturating_add(6);
        }
        x
    }

    pub async fn flush(&mut self) -> Result<(), Driver::Error> {
        self.send_command(&[0x21, 0, 127]).await?; // cols 0..127
        self.send_command(&[0x22, 0, 7]).await?; // pages 0..7

        let mut buf = [0u8; 129];
        buf[0] = 0x40;
        for page in 0..SSD1306_PAGES {
            let start = page * SSD1306_WIDTH;
            let end = start + SSD1306_WIDTH;
            buf[1..129].copy_from_slice(&self.frame_buf[start..end]);
            self.i2c.write(self.addr, &buf).await?;
        }
        Ok(())
    }
}
