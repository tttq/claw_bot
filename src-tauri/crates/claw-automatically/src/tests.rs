#[cfg(test)]
mod tests {
    use crate::types::ImageFrame;
    use crate::AutomaticallyError;

    #[test]
    fn test_validate_coordinates_valid() {
        let (screen_w, screen_h) =
            crate::platform::window::get_screen_size().unwrap_or((1920, 1080));
        let x = (screen_w / 2) as f64;
        let y = (screen_h / 2) as f64;
        assert!(crate::coordinate_validator::validate_coordinates(x, y).is_ok());
    }

    #[test]
    fn test_validate_coordinates_negative() {
        let result = crate::coordinate_validator::validate_coordinates(-1.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_coordinates_out_of_bounds() {
        let (screen_w, screen_h) =
            crate::platform::window::get_screen_size().unwrap_or((1920, 1080));
        let result =
            crate::coordinate_validator::validate_coordinates(screen_w as f64, screen_h as f64);
        assert!(result.is_err());
    }

    #[test]
    fn test_clamp_coordinates_negative() {
        let (clamped_x, clamped_y) =
            crate::coordinate_validator::validate_and_clamp(-100.0, -50.0);
        assert!(clamped_x >= 0.0);
        assert!(clamped_y >= 0.0);
    }

    #[test]
    fn test_clamp_coordinates_valid() {
        let (clamped_x, clamped_y) =
            crate::coordinate_validator::validate_and_clamp(500.0, 300.0);
        assert_eq!(clamped_x, 500.0);
        assert_eq!(clamped_y, 300.0);
    }

    #[test]
    fn test_screenshot_diff_identical() {
        let frame1 = ImageFrame::new(20, 20, vec![0u8; 1200]);
        let frame2 = ImageFrame::new(20, 20, vec![0u8; 1200]);
        let diff = crate::screenshot_diff::compare_screenshots(&frame1, &frame2).unwrap();
        assert_eq!(diff.pixel_diff_count, 0);
        assert_eq!(diff.similarity_percent, 100.0);
    }

    #[test]
    fn test_screenshot_diff_different() {
        let frame1 = ImageFrame::new(10, 10, vec![0u8; 300]);
        let mut frame2_data = vec![255u8; 300];
        frame2_data[0] = 0;
        let frame2 = ImageFrame::new(10, 10, frame2_data);
        let diff = crate::screenshot_diff::compare_screenshots(&frame1, &frame2).unwrap();
        assert!(diff.pixel_diff_count > 0);
        assert!(diff.similarity_percent < 100.0);
    }

    #[test]
    fn test_screenshot_diff_different_size() {
        let frame1 = ImageFrame::new(10, 10, vec![0u8; 100]);
        let frame2 = ImageFrame::new(10, 20, vec![0u8; 200]);
        let result = crate::screenshot_diff::compare_screenshots(&frame1, &frame2);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display() {
        let err = AutomaticallyError::InvalidCoordinates(0.0, 0.0, 1920, 1080);
        assert!(err.to_string().contains("Invalid coordinates"));

        let err = AutomaticallyError::Timeout("test timeout".to_string());
        assert!(err.to_string().contains("Timeout"));

        let err = AutomaticallyError::Clipboard("clipboard error".to_string());
        assert!(err.to_string().contains("Clipboard"));

        let err = AutomaticallyError::MaxRetriesExceeded("max retries".to_string());
        assert!(err.to_string().contains("Max retries"));

        let err = AutomaticallyError::Verify("verify failed".to_string());
        assert!(err.to_string().contains("verify failed"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let auto_err: AutomaticallyError = io_err.into();
        assert!(auto_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_from_json() {
        let json = r#"{"invalid": json}"#;
        let json_err = serde_json::from_str::<serde_json::Value>(json).unwrap_err();
        let auto_err: AutomaticallyError = json_err.into();
        assert!(!auto_err.to_string().is_empty());
    }

    #[test]
    fn test_screenshot_diff_has_changed() {
        let frame1 = ImageFrame::new(10, 10, vec![0u8; 300]);
        let frame2 = ImageFrame::new(10, 10, vec![255u8; 300]);
        let changed = crate::screenshot_diff::has_screen_changed(&frame1, &frame2).unwrap();
        assert!(changed);
    }

    #[test]
    fn test_screenshot_diff_not_changed() {
        let frame1 = ImageFrame::new(10, 10, vec![42u8; 300]);
        let frame2 = ImageFrame::new(10, 10, vec![42u8; 300]);
        let changed = crate::screenshot_diff::has_screen_changed(&frame1, &frame2).unwrap();
        assert!(!changed);
    }
}
