use image::{imageops::FilterType, DynamicImage, GrayImage};
use tracing::trace;

const HISTOGRAM_BINS: usize = 256;
const HISTOGRAM_THRESHOLD: f32 = 0.08; // total variation distance threshold
const SSIM_THRESHOLD: f32 = 0.92; // minimum acceptable similarity score
const SSIM_SAMPLE_SIZE: u32 = 96; // downsample target edge for SSIM calculation

#[derive(Clone)]
struct FrameSignature {
    histogram: [u32; HISTOGRAM_BINS],
    ssim_sample: Vec<u8>,
}

impl FrameSignature {
    fn from_image(image: &DynamicImage) -> Self {
        let grayscale = image.to_luma8();
        let histogram = build_histogram(&grayscale);
        let downsampled = downsample_for_ssim(&grayscale);

        Self {
            histogram,
            ssim_sample: downsampled,
        }
    }
}

#[derive(Default)]
pub struct ChangeDetector {
    previous: Option<FrameSignature>,
}

#[derive(Debug, Clone)]
pub enum ChangeDecision {
    FirstFrame,
    Significant {
        histogram_delta: f32,
        ssim_score: f32,
    },
    Insignificant {
        histogram_delta: f32,
        ssim_score: f32,
    },
}

impl ChangeDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn evaluate(&mut self, image: &DynamicImage) -> ChangeDecision {
        let signature = FrameSignature::from_image(image);

        match &self.previous {
            None => {
                self.previous = Some(signature);
                ChangeDecision::FirstFrame
            }
            Some(previous_signature) => {
                let histogram_delta =
                    histogram_distance(&signature.histogram, &previous_signature.histogram);

                let ssim_score =
                    compute_ssim(&signature.ssim_sample, &previous_signature.ssim_sample);

                trace!(histogram_delta, ssim_score, "frame diff metrics computed");

                self.previous = Some(signature);

                if histogram_delta >= HISTOGRAM_THRESHOLD || ssim_score <= SSIM_THRESHOLD {
                    ChangeDecision::Significant {
                        histogram_delta,
                        ssim_score,
                    }
                } else {
                    ChangeDecision::Insignificant {
                        histogram_delta,
                        ssim_score,
                    }
                }
            }
        }
    }
}

fn histogram_distance(current: &[u32; HISTOGRAM_BINS], previous: &[u32; HISTOGRAM_BINS]) -> f32 {
    let current_total = current.iter().sum::<u32>().max(1) as f32;
    let previous_total = previous.iter().sum::<u32>().max(1) as f32;

    current
        .iter()
        .zip(previous.iter())
        .map(|(c, p)| {
            let current_ratio = *c as f32 / current_total;
            let prev_ratio = *p as f32 / previous_total;
            (current_ratio - prev_ratio).abs()
        })
        .sum::<f32>()
        * 0.5
}

fn compute_ssim(current: &[u8], previous: &[u8]) -> f32 {
    if current.is_empty() || previous.is_empty() || current.len() != previous.len() {
        return 1.0;
    }

    let n = current.len() as f64;
    let mean_current = current.iter().map(|&v| v as f64).sum::<f64>() / n;
    let mean_previous = previous.iter().map(|&v| v as f64).sum::<f64>() / n;

    let variance_current = current
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean_current;
            diff * diff
        })
        .sum::<f64>()
        / n;

    let variance_previous = previous
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean_previous;
            diff * diff
        })
        .sum::<f64>()
        / n;

    let covariance = current
        .iter()
        .zip(previous.iter())
        .map(|(&a, &b)| (a as f64 - mean_current) * (b as f64 - mean_previous))
        .sum::<f64>()
        / n;

    let c1: f64 = (0.01f64 * 255.0f64).powi(2);
    let c2: f64 = (0.03f64 * 255.0f64).powi(2);

    let numerator = (2.0 * mean_current * mean_previous + c1) * (2.0 * covariance + c2);
    let denominator = (mean_current.powi(2) + mean_previous.powi(2) + c1)
        * (variance_current + variance_previous + c2);

    if denominator.abs() < f64::EPSILON {
        1.0
    } else {
        (numerator / denominator).clamp(-1.0, 1.0) as f32
    }
}

fn build_histogram(image: &GrayImage) -> [u32; HISTOGRAM_BINS] {
    let mut bins = [0u32; HISTOGRAM_BINS];
    for pixel in image.iter() {
        bins[*pixel as usize] += 1;
    }
    bins
}

fn downsample_for_ssim(image: &GrayImage) -> Vec<u8> {
    if image.width() <= SSIM_SAMPLE_SIZE && image.height() <= SSIM_SAMPLE_SIZE {
        return image.to_vec();
    }

    let resized = image::DynamicImage::ImageLuma8(image.clone()).resize(
        SSIM_SAMPLE_SIZE,
        SSIM_SAMPLE_SIZE,
        FilterType::Triangle,
    );

    resized.to_luma8().into_raw()
}
