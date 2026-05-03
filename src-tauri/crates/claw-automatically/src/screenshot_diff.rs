use crate::error::{AutomaticallyError, Result};
use crate::types::ImageFrame;

pub struct ScreenshotDiff {
    pub pixel_diff_count: u64,
    pub similarity_percent: f64,
    pub has_changed: bool,
    pub change_threshold: f64,
}

impl ScreenshotDiff {
    pub fn new(
        pixel_diff_count: u64,
        total_pixels: u64,
        change_threshold: f64,
    ) -> Self {
        let similarity_percent = if total_pixels > 0 {
            ((total_pixels - pixel_diff_count) as f64 / total_pixels as f64) * 100.0
        } else {
            100.0
        };

        Self {
            pixel_diff_count,
            similarity_percent,
            has_changed: similarity_percent < (100.0 - change_threshold * 100.0),
            change_threshold,
        }
    }
}

pub fn compare_screenshots(before: &ImageFrame, after: &ImageFrame) -> Result<ScreenshotDiff> {
    if before.width != after.width || before.height != after.height {
        return Err(AutomaticallyError::Capture(
            "Screenshots have different dimensions".to_string(),
        ));
    }

    let total_pixels = (before.width as u64) * (before.height as u64);
    let mut diff_count: u64 = 0;

    let min_len = before.data.len().min(after.data.len());
    let pixel_size = 3;

    for chunk_start in (0..min_len).step_by(pixel_size) {
        let end = (chunk_start + pixel_size).min(min_len);
        if before.data[chunk_start..end] != after.data[chunk_start..end] {
            diff_count += 1;
        }
    }

    let diff = ScreenshotDiff::new(diff_count, total_pixels, 0.01);

    log::info!(
        "[ScreenshotDiff] Comparison: {} diff pixels / {} total = {:.2}% similarity (changed: {})",
        diff.pixel_diff_count,
        total_pixels,
        diff.similarity_percent,
        diff.has_changed
    );

    Ok(diff)
}

pub fn has_screen_changed(before: &ImageFrame, after: &ImageFrame) -> Result<bool> {
    let diff = compare_screenshots(before, after)?;
    Ok(diff.has_changed)
}

pub fn verify_operation_effect(
    screenshot_before: &ImageFrame,
    screenshot_after: &ImageFrame,
    min_change_percent: f64,
) -> Result<bool> {
    let diff = compare_screenshots(screenshot_before, screenshot_after)?;
    let change_percent = 100.0 - diff.similarity_percent;

    if change_percent < min_change_percent {
        log::warn!(
            "[ScreenshotDiff] Operation may have had no visible effect: {:.2}% change (min: {:.2}%)",
            change_percent,
            min_change_percent
        );
        Ok(false)
    } else {
        log::info!(
            "[ScreenshotDiff] Operation had visible effect: {:.2}% change",
            change_percent
        );
        Ok(true)
    }
}
