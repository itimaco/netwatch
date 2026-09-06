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
