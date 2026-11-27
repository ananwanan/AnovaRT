// SPDX-License-Identifier: Apache-2.0
//! Experimental ILI9341/FSMC NE4 demo; verify the board wiring before use.
use core::ptr::{read_volatile, write_volatile};

const RCC: usize = 0x4002_1000;
const GPIOB: usize = 0x4001_0C00;
const GPIOD: usize = 0x4001_1400;
const GPIOE: usize = 0x4001_1800;
const GPIOG: usize = 0x4001_2000;
const FSMC_BCR4: usize = 0xA000_0018;
const FSMC_BTR4: usize = 0xA000_001C;
const LCD_CMD: *mut u16 = 0x6C00_0000 as *mut u16;
const LCD_DATA: *mut u16 = 0x6C00_0800 as *mut u16;
const WIDTH: u16 = 240;
const HEIGHT: u16 = 320;

fn read(addr: usize) -> u32 {
    // SAFETY: Only aligned, valid MMIO register addresses are passed here.
    unsafe { read_volatile(addr as *const u32) }
}
fn write(addr: usize, value: u32) {
    // SAFETY: The demo exclusively owns these aligned MMIO registers.
    unsafe { write_volatile(addr as *mut u32, value) }
}
fn modify(addr: usize, clear: u32, set: u32) {
    write(addr, (read(addr) & !clear) | set);
}
fn gpio(port: usize, pin: u32, mode: u32) {
    let addr = port + if pin < 8 { 0 } else { 4 };
    let shift = (pin % 8) * 4;
    modify(addr, 0xF << shift, mode << shift);
}
fn backlight(on: bool) {
    write(GPIOB + 0x10, if on { 1 } else { 1 << 16 });
}
fn fsmc_init() {
    modify(RCC + 0x18, 0, (1 << 3) | (1 << 5) | (1 << 6) | (1 << 8));
    modify(RCC + 0x14, 0, 1 << 8);
    let _ = read(RCC + 0x14);
    backlight(false);
    gpio(GPIOB, 0, 0x2);
    for pin in [0, 1, 4, 5, 8, 9, 10, 14, 15] {
        gpio(GPIOD, pin, 0xB);
    }
    for pin in 7..=15 {
        gpio(GPIOE, pin, 0xB);
    }
    for pin in [0, 12] {
        gpio(GPIOG, pin, 0xB);
    }
    // Asynchronous SRAM-like 16-bit bus, write enable, conservative timing.
    write(FSMC_BCR4, (1 << 12) | (1 << 4));
    write(FSMC_BTR4, 7 | (27 << 8) | (1 << 16));
    modify(FSMC_BCR4, 0, 1);
}
fn cmd(value: u8) {
    // SAFETY: The configured FSMC NE4 bank maps this aligned halfword.
    unsafe { write_volatile(LCD_CMD, value as u16) }
}
fn data(value: u16) {
    // SAFETY: The configured FSMC NE4 bank maps this aligned halfword.
    unsafe { write_volatile(LCD_DATA, value) }
}
fn command(value: u8, params: &[u8]) {
    cmd(value);
    for &p in params {
        data(p as u16);
    }
}
fn init(delay_ms: impl Fn(u32)) {
    delay_ms(20);
    cmd(0x01);
    delay_ms(150);
    cmd(0x28);
    command(0xCF, &[0x00, 0xC1, 0x30]);
    command(0xED, &[0x64, 0x03, 0x12, 0x81]);
    command(0xE8, &[0x85, 0x00, 0x78]);
    command(0xCB, &[0x39, 0x2C, 0x00, 0x34, 0x02]);
    command(0xF7, &[0x20]);
    command(0xEA, &[0x00, 0x00]);
    command(0xC0, &[0x23]);
    command(0xC1, &[0x10]);
    command(0xC5, &[0x3E, 0x28]);
    command(0xC7, &[0x86]);
    command(0x36, &[0x48]);
    command(0x3A, &[0x55]);
    command(0xB1, &[0x00, 0x18]);
    command(0xB6, &[0x08, 0x82, 0x27]);
    command(0xF2, &[0x00]);
    command(0x26, &[0x01]);
    cmd(0x11);
    delay_ms(120);
    cmd(0x29);
    delay_ms(20);
}
const fn rgb(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | (b as u16 >> 3)
}
fn rect(x: u16, y: u16, w: u16, h: u16, color: u16) {
    if w == 0 || h == 0 || x >= WIDTH || y >= HEIGHT {
        return;
    }
    let w = w.min(WIDTH - x);
    let h = h.min(HEIGHT - y);
    let x1 = x + w - 1;
    let y1 = y + h - 1;
    command(0x2A, &[(x >> 8) as u8, x as u8, (x1 >> 8) as u8, x1 as u8]);
    command(0x2B, &[(y >> 8) as u8, y as u8, (y1 >> 8) as u8, y1 as u8]);
    cmd(0x2C);
    for _ in 0..(w as u32 * h as u32) {
        data(color);
    }
}
const CAT: [&[u8]; 19] = [
    b"................",
    b"..#.........#...",
    b"..##.......##...",
    b"..#p#.....#p#...",
    b"..#pp#####pp#...",
    b"..##KKWWKK##....",
    b"..#KKKWWKKK#....",
    b"..#KKWWWWKK#....",
    b"..#KOWWWWOK#....",
    b"..#WWWWWWWW#....",
    b"..#pWWnWWWp#....",
    b"...#WWWWWW#.....",
    b"....######......",
    b"....#KWWK#...##.",
    b"...#KKWWKK#..#K#",
    b"...#KWWWWK###K#.",
    b"...#WWWWWWKKK#..",
    b"....##..####....",
    b"................",
];
fn glyph(c: u8) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        b'A' => [14, 17, 17, 31, 17, 17, 17],
        b'N' => [17, 25, 21, 19, 17, 17, 17],
        b'O' => [14, 17, 17, 17, 17, 17, 14],
        b'V' => [17, 17, 17, 17, 17, 10, 4],
        b'R' => [30, 17, 17, 30, 20, 18, 17],
        b'T' => [31, 4, 4, 4, 4, 4, 4],
        b'L' => [16, 16, 16, 16, 16, 16, 31],
        b'C' => [14, 17, 16, 16, 16, 17, 14],
        b'D' => [30, 17, 17, 17, 17, 17, 30],
        b'P' => [30, 17, 17, 30, 16, 16, 16],
        b'I' => [31, 4, 4, 4, 4, 4, 31],
        b'X' => [17, 17, 10, 4, 10, 17, 17],
        b'E' => [31, 16, 16, 30, 16, 16, 31],
        b'M' => [17, 27, 21, 21, 17, 17, 17],
        _ => [0; 7],
    }
}
fn centered_text(y: u16, s: &[u8], scale: u16, color: u16) {
    let width = (s.len() as u16 * 6 - 1) * scale;
    let x0 = (WIDTH - width) / 2;
    for (i, &c) in s.iter().enumerate() {
        for (y1, row) in glyph(c).iter().enumerate() {
            for x1 in 0..5u16 {
                if row & (1 << (4 - x1)) != 0 {
                    rect(
                        x0 + (i as u16 * 6 + x1) * scale,
                        y + y1 as u16 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }
}
fn draw_cat() {
    rect(0, 0, WIDTH, HEIGHT, rgb(252, 232, 220));
    for (y, row) in CAT.iter().enumerate() {
        for (x, &p) in row.iter().enumerate() {
            let color = match p {
                b'#' => rgb(39, 37, 46),
                b'K' => rgb(57, 54, 66),
                b'W' => rgb(255, 249, 239),
                b'p' => rgb(244, 155, 170),
                b'O' => rgb(139, 191, 101),
                b'n' => rgb(220, 116, 138),
                _ => continue,
            };
            rect(48 + x as u16 * 9, 55 + y as u16 * 9, 9, 9, color);
        }
    }
    centered_text(240, b"AnovaRT", 2, rgb(55, 51, 59));
    centered_text(263, b"LCD PIXEL CAT DEMO", 1, rgb(55, 51, 59));
    for (i, color) in [0xF800, 0x07E0, 0x001F, 0xFFFF, 0x0000, 0xFFE0, 0x07FF, 0xF81F]
        .into_iter()
        .enumerate()
    {
        rect(i as u16 * 30, 298, 30, 22, color);
    }
}

/// Run the solid-color tests and draw the final static picture.
///
/// # Safety
/// Call once with interrupts masked and exclusive ownership of RCC, FSMC NE4
/// and all candidate LCD GPIO pins. The operator must verify the exact LCD
/// wiring, backlight polarity and ILI9341 compatibility before flashing.
/// The delay callback must provide at least the requested milliseconds.
pub unsafe fn run(delay_ms: impl Fn(u32)) {
    fsmc_init();
    init(&delay_ms);
    backlight(true);
    for color in [0xF800, 0x07E0, 0x001F, 0xFFFF, 0x0000] {
        rect(0, 0, WIDTH, HEIGHT, color);
        delay_ms(250);
    }
    draw_cat();
}
