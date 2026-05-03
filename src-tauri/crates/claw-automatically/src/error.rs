use thiserror::Error;

pub type Result<T> = std::result::Result<T, AutomaticallyError>;

#[derive(Error, Debug, Clone)]
pub enum AutomaticallyError {
    #[error("OCR error: {0}")]
    Ocr(String),

    #[error("Screen capture error: {0}")]
    Capture(String),

    #[error("Input simulation error: {0}")]
    Input(String),

    #[error("File processing error: {0}")]
    FileProcessing(String),

    #[error("Automation error: {0}")]
    Automation(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("Invalid coordinates: ({0}, {1}) — screen bounds: ({2}, {3})")]
    InvalidCoordinates(f64, f64, u32, u32),

    #[error("Mano-P model error: {0}")]
    ManoP(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Inference engine error: {0}")]
    InferenceEngine(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Window not found: {0}")]
    WindowNotFound(String),

    #[error("Application not found: {0}")]
    AppNotFound(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("File handling error: {0}")]
    FileHandling(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Max retries exceeded: {0}")]
    MaxRetriesExceeded(String),

    #[error("Clipboard error: {0}")]
    Clipboard(String),

    #[error("Operation verify failed: {0}")]
    Verify(String),

    #[error("Coordinate out of screen bounds: ({0}, {1})")]
    OutOfBounds(f64, f64),
}

impl From<std::io::Error> for AutomaticallyError {
    fn from(e: std::io::Error) -> Self {
        AutomaticallyError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AutomaticallyError {
    fn from(e: serde_json::Error) -> Self {
        AutomaticallyError::Json(e.to_string())
    }
}
