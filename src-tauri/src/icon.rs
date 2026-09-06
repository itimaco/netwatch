use tauri::image::Image;

const S: usize = 32;

#[derive(Clone, Copy, PartialEq)]
pub enum Shape { Solid, Ring }

pub fn make(color: [u8; 4], shape: Shape, spinner_angle: Option<f32>) -> Image<'static> {
	let mut buffer = vec![0u8; S * S * 4];
	let center = (S as f32 - 1.0) / 2.0;
	let radius = 13.0;
	let inner = if shape == Shape::Ring { 7.0 } else { -1.0 };
	for y in 0..S {
		for x in 0..S {
			let distance = ((x as f32 - center).powi(2) + (y as f32 - center).powi(2)).sqrt();
			let mut alpha = if distance <= radius - 1.0 { 1.0 } else if distance < radius { radius - distance } else { 0.0 };
			if inner > 0.0 { alpha *= if distance >= inner + 1.0 { 1.0 } else if distance > inner { distance - inner } else { 0.0 }; }
			if alpha > 0.0 {
				let index = (y * S + x) * 4;
				buffer[index] = color[0]; buffer[index + 1] = color[1]; buffer[index + 2] = color[2]; buffer[index + 3] = (255.0 * alpha) as u8;
			}
		}
	}
	if let Some(angle) = spinner_angle {
		let (point_x, point_y) = (center + 8.0 * angle.cos(), center + 8.0 * angle.sin());
		for y in 0..S { for x in 0..S { if (x as f32 - point_x).powi(2) + (y as f32 - point_y).powi(2) <= 9.0 { let index = (y * S + x) * 4; buffer[index..index + 4].copy_from_slice(&[255, 255, 255, 255]); } } }
	}
	Image::new_owned(buffer, S as u32, S as u32)
}

const GLYPH: [[u8; 5]; 10] = [
	[0b111, 0b101, 0b101, 0b101, 0b111], [0b010, 0b110, 0b010, 0b010, 0b111],
	[0b111, 0b001, 0b111, 0b100, 0b111], [0b111, 0b001, 0b111, 0b001, 0b111],
	[0b101, 0b101, 0b111, 0b001, 0b001], [0b111, 0b100, 0b111, 0b001, 0b111],
	[0b111, 0b100, 0b111, 0b101, 0b111], [0b111, 0b001, 0b001, 0b010, 0b010],
	[0b111, 0b101, 0b111, 0b101, 0b111], [0b111, 0b101, 0b111, 0b001, 0b111],
];
const ARROW_DOWN: [u8; 5] = [0b010, 0b010, 0b010, 0b111, 0b010];
const ARROW_UP: [u8; 5] = [0b010, 0b111, 0b010, 0b010, 0b010];
const TEXT: [u8; 4] = [240, 240, 245, 255];
const DL: [u8; 4] = [74, 222, 128, 255];
const UL: [u8; 4] = [96, 165, 250, 255];
const BASE: usize = 16;

pub fn format_mb(bps: u64) -> String {
	let megabytes = bps as f64 / (1024.0 * 1024.0);
	if megabytes >= 10.0 { format!("{:.0}", megabytes.min(999.0)) } else { format!("{:.1}", megabytes) }
}

fn text_width(text: &str) -> usize { text.chars().map(|c| if c == '.' { 1 } else { 3 }).sum::<usize>() + text.chars().count().saturating_sub(1) }
fn blit(grid: &mut [[Option<[u8; 4]>; BASE]; BASE], rows: &[u8; 5], x: usize, y: usize, color: [u8; 4]) { for (dy, row) in rows.iter().enumerate() { for dx in 0..3 { if row & (0b100 >> dx) != 0 { if let Some(cell) = grid.get_mut(y + dy).and_then(|line| line.get_mut(x + dx)) { *cell = Some(color); } } } } }

type Grid = [[Option<[u8; 4]>; BASE]; BASE];
fn render(grid: &Grid, scale: usize) -> Image<'static> { let scale = scale.clamp(1, 4); let size = BASE * scale; let mut buffer = vec![0u8; size * size * 4]; for y in 0..size { for x in 0..size { if let Some(color) = grid[y / scale][x / scale] { let index = (y * size + x) * 4; buffer[index..index + 4].copy_from_slice(&color); } } } Image::new_owned(buffer, size as u32, size as u32) }
fn draw_line_at(grid: &mut Grid, arrow: &[u8; 5], arrow_color: [u8; 4], text: &str, text_color: [u8; 4], y: usize, right: usize) { blit(grid, arrow, 1, y, arrow_color); let mut x = right - text_width(text); for character in text.chars() { if character == '.' { if let Some(cell) = grid[y + 4].get_mut(x) { *cell = Some(text_color); } x += 2; } else if let Some(digit) = character.to_digit(10) { blit(grid, &GLYPH[digit as usize], x, y, text_color); x += 4; } } }
pub fn speed_text(down: &str, up: &str, scale: usize) -> Image<'static> { let mut grid: Grid = [[None; BASE]; BASE]; draw_line_at(&mut grid, &ARROW_DOWN, DL, down, TEXT, 2, BASE); draw_line_at(&mut grid, &ARROW_UP, UL, up, TEXT, 9, BASE); render(&grid, scale) }
pub fn badge(down: &str, up: &str, background: [u8; 4], scale: usize) -> Image<'static> { let luminance = 0.2126 * background[0] as f32 + 0.7152 * background[1] as f32 + 0.0722 * background[2] as f32; let text_color = if luminance > 150.0 { [12, 12, 16, 255] } else { [255, 255, 255, 255] }; let mut grid: Grid = [[Some(background); BASE]; BASE]; for &(x, y) in &[(0, 0), (BASE - 1, 0), (0, BASE - 1), (BASE - 1, BASE - 1)] { grid[y][x] = None; } draw_line_at(&mut grid, &ARROW_DOWN, text_color, down, text_color, 2, BASE - 1); draw_line_at(&mut grid, &ARROW_UP, text_color, up, text_color, 9, BASE - 1); render(&grid, scale) }
