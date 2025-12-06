//! OCR abstraction layer powered by Windows-native APIs first.
//!
//! The Windows implementation uses `Windows.Media.Ocr` to perform on-device OCR.

use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

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
    async fn recognize(&self, image_bytes: &[u8], context: &OcrContext) -> Result<OcrPayload> {
        #[cfg(target_os = "windows")]
        {
            let (text, json) = ocr_windows(image_bytes, context).await?;
            Ok(OcrPayload {
                text,
                confidence: None,
                json: Some(json),
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            Ok(OcrPayload {
                text: format!("[stub ocr for {}]", context.window_name),
                confidence: None,
                json: None,
            })
        }
    }

    fn name(&self) -> &'static str {
        "windows-ocr"
    }
}

#[cfg(target_os = "windows")]
async fn ocr_windows(image_bytes: &[u8], context: &OcrContext) -> Result<(String, String)> {
    use anyhow::Context as _;
    use windows::{
        core::HSTRING,
        Globalization::Language,
        Graphics::Imaging::{BitmapDecoder, BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
    };

    // Write bytes into an in-memory stream for the decoder.
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::new()?;
    writer.WriteBytes(image_bytes)?;
    writer.StoreAsync()?.GetResults()?;
    let buffer = writer.DetachBuffer()?;
    stream.WriteAsync(&buffer)?.GetResults()?;
    stream.Seek(0)?;

    let decoder = BitmapDecoder::CreateAsync(&stream)?.GetResults()?;
    let bitmap = decoder.GetSoftwareBitmapAsync()?.GetResults()?;
    // Ensure format is supported by OCR (BGRA8).
    let bitmap = SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)?;

    let engine = if let Some(lang) = context
        .languages
        .iter()
        .find_map(|l| Language::CreateLanguage(&HSTRING::from(l)).ok())
    {
        OcrEngine::TryCreateFromLanguage(&lang)?
    } else {
        OcrEngine::TryCreateFromUserProfileLanguages()?
    };

    let result = engine
        .RecognizeAsync(&bitmap)?
        .GetResults()
        .context("Windows OCR recognize failed")?;

    let text = result.Text()?.to_string_lossy();
    let json = format!(
        r#"{{"engine":"{}","window":"{}","app":"{}","lang":"{}"}}"#,
        "windows.media.ocr",
        context.window_name,
        context.app_name,
        engine
            .RecognizerLanguage()?
            .LanguageTag()?
            .to_string_lossy()
    );

    debug!(
        window = %context.window_name,
        app = %context.app_name,
        lang = %engine.RecognizerLanguage()?.LanguageTag()?.to_string_lossy(),
        "windows ocr completed"
    );

    Ok((text, json))
}
