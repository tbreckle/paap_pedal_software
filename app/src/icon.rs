//! Icon generation module for the PAAP Configurator application.
//!
//! This module provides functionality to generate a custom application icon
//! programmatically, which is used in the window title bar and taskbar.

use eframe::egui;

/// Creates a 32x32 keyboard icon for the application.
///
/// Generates a stylized keyboard icon with white keys on a blue background.
/// The icon is rendered as a 32x32 RGBA bitmap and is suitable for use as
/// a window icon in the application.
///
/// # Returns
///
/// An `egui::IconData` struct containing the RGBA pixel data and dimensions
/// of the generated keyboard icon.
pub fn create_icon() -> egui::IconData {
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];

    // Background color (light blue/gray).
    for pixel in rgba.chunks_mut(4) {
        pixel[0] = 70; // R
        pixel[1] = 130; // G
        pixel[2] = 180; // B
        pixel[3] = 255; // A
    }

    // Draw keyboard keys (simplified representation).
    let draw_rect = |rgba: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: [u8; 4]| {
        for py in y..(y + h).min(size) {
            for px in x..(x + w).min(size) {
                let idx = (py * size + px) * 4;
                if idx + 3 < rgba.len() {
                    rgba[idx] = color[0];
                    rgba[idx + 1] = color[1];
                    rgba[idx + 2] = color[2];
                    rgba[idx + 3] = color[3];
                }
            }
        }
    };

    let white = [240, 240, 240, 255];
    let dark = [40, 40, 40, 255];

    // Draw border.
    for i in 0..size {
        draw_rect(&mut rgba, i, 0, 1, 1, dark);
        draw_rect(&mut rgba, i, size - 1, 1, 1, dark);
        draw_rect(&mut rgba, 0, i, 1, 1, dark);
        draw_rect(&mut rgba, size - 1, i, 1, 1, dark);
    }

    // Draw keys (3 rows of keys).
    for row in 0..3 {
        for col in 0..5 {
            let x = 4 + col * 5;
            let y = 8 + row * 7;
            draw_rect(&mut rgba, x, y, 4, 5, white);
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
