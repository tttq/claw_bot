use crate::error::{AutomaticallyError, Result};
use crate::platform::window;

pub fn validate_coordinates(x: f64, y: f64) -> Result<()> {
    let (screen_w, screen_h) = window::get_screen_size()
        .map_err(|e| AutomaticallyError::Capture(format!("Failed to get screen size: {}", e)))?;

    if x < 0.0 || y < 0.0 || x >= screen_w as f64 || y >= screen_h as f64 {
        return Err(AutomaticallyError::InvalidCoordinates(
            x, y, screen_w, screen_h,
        ));
    }

    Ok(())
}

pub fn clamp_coordinates(x: f64, y: f64) -> (f64, f64) {
    let (screen_w, screen_h) = window::get_screen_size().unwrap_or((1920, 1080));
    let clamped_x = x.clamp(0.0, (screen_w as f64) - 1.0);
    let clamped_y = y.clamp(0.0, (screen_h as f64) - 1.0);
    (clamped_x, clamped_y)
}

pub fn validate_and_clamp(x: f64, y: f64) -> (f64, f64) {
    let (clamped_x, clamped_y) = clamp_coordinates(x, y);
    if clamped_x != x || clamped_y != y {
        log::warn!(
            "[CoordinateValidator] Coordinates clamped from ({}, {}) to ({}, {})",
            x, y, clamped_x, clamped_y
        );
    }
    (clamped_x, clamped_y)
}
