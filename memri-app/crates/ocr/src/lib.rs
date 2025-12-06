//! OCR abstraction layer powered by Windows-native APIs first.
//!
//! Implementations will wrap native Windows OCR, Tesseract, or cloud engines.

use anyhow::Result;
use async_trait::async_trait;

/// OCR text output including optional structured details.
#[derive(Debug, Clone)]
pub struct OcrPayload {
    pub text: String,
    pub confidence: Option<f32>,
    pub json: Option<String>,
}

/// Metadata about the window/surface being processed.
#[derive(Debug, Clone)]
pub struct OcrContext {
    pub window_name: String,
    pub app_name: String,
    pub is_focused: bool,
    pub languages: Vec<String>,
}

/// Trait that all OCR engines must implement.
#[async_trait]
pub trait OcrEngine: Send + Sync {
    async fn recognize(&self, image_bytes: &[u8], context: &OcrContext) -> Result<OcrPayload>;

    fn name(&self) -> &'static str;
}

/// Placeholder Windows OCR implementation.
pub struct WindowsOcr;

#[async_trait]
impl OcrEngine for WindowsOcr {
    async fn recognize(&self, _image_bytes: &[u8], context: &OcrContext) -> Result<OcrPayload> {
        // Real implementation will bridge to Windows.Media.Ocr.
        Ok(OcrPayload {
            text: format!("[stub ocr for {}]", context.window_name),
            confidence: None,
            json: None,
        })
    }

    fn name(&self) -> &'static str {
        "windows-ocr"
    }
}
