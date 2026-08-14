use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView};
use std::path::Path;
use webp::Encoder;

use crate::{CompressionMode, OutputFormat};

/// Estimate the compressed size for a given original size, mode, quality and
/// output format. The single estimator shared by dry-run reporting and the
/// GUI preview.
pub fn estimate_output_size(
    original_size: u64,
    mode: &CompressionMode,
    quality: u8,
    output_format: &OutputFormat,
) -> u64 {
    let factor = match mode {
        CompressionMode::Lossless => 0.7, // Lossless typically saves 20-30%
        CompressionMode::Lossy => match quality {
            90..=100 => 0.6,
            70..=89 => 0.4,
            50..=69 => 0.3,
            _ => 0.2,
        },
        CompressionMode::Auto => 0.5, // Conservative estimate for auto mode
    };
    // AVIF typically lands 30-50% below WebP at the same quality
    let factor = if output_format == &OutputFormat::Avif {
        factor * 0.6
    } else {
        factor
    };
    (original_size as f64 * factor) as u64
}

pub struct ImageConverter {
    quality: f32,
    mode: CompressionMode,
    output_format: OutputFormat,
    // Ultra-fast mode for maximum performance
    ultra_fast: bool,
    // Dry run mode - preview without actual conversion
    dry_run: bool,
}

impl ImageConverter {
    pub fn new_with_dry_run(
        quality: u8,
        mode: &CompressionMode,
        output_format: &OutputFormat,
        dry_run: bool,
    ) -> Self {
        Self {
            quality: quality as f32,
            mode: mode.clone(),
            output_format: output_format.clone(),
            ultra_fast: true,
            dry_run,
        }
    }

    pub fn convert_to_webp(&self, input_path: &Path, output_path: &Path) -> Result<(u64, u64)> {
        let original_size = std::fs::metadata(input_path)?.len();

        // Dry run mode: only analyze without converting
        if self.dry_run {
            self.analyze_conversion(input_path, output_path)?;
            return Ok((
                original_size,
                estimate_output_size(
                    original_size,
                    &self.mode,
                    self.quality as u8,
                    &self.output_format,
                ),
            ));
        }

        // Performance: Read image with optimized buffer size
        let img = image::open(input_path)
            .with_context(|| format!("Failed to read image: {}", input_path.display()))?;

        // Validate and potentially resize image to fit WebP constraints
        let processed_img = match self.validate_and_resize_image(&img)? {
            Some(resized) => resized,
            None => img, // Use original image without cloning
        };

        // Choose conversion strategy based on mode
        match self.mode {
            CompressionMode::Lossless | CompressionMode::Lossy => {
                self.encode_output(&processed_img, output_path, &self.mode)
            }
            CompressionMode::Auto => {
                self.convert_auto_fast(&processed_img, output_path, input_path)
            }
        }?;

        let compressed_size = std::fs::metadata(output_path)?.len();
        Ok((original_size, compressed_size))
    }

    /// Analyze conversion without actually performing it (dry run mode)
    fn analyze_conversion(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        // Read image to analyze but don't convert
        let img = image::open(input_path)
            .with_context(|| format!("Failed to read image: {}", input_path.display()))?;

        let (width, height) = img.dimensions();
        let compression_mode = if matches!(self.mode, CompressionMode::Auto) {
            if self.should_use_lossless_fast(&img, input_path) {
                "lossless"
            } else {
                "lossy"
            }
        } else {
            match self.mode {
                CompressionMode::Lossless => "lossless",
                CompressionMode::Lossy => "lossy",
                CompressionMode::Auto => unreachable!(),
            }
        };

        log::info!(
            "[DRY RUN] {} -> {} ({}x{}, mode: {}, quality: {})",
            input_path.display(),
            output_path.display(),
            width,
            height,
            compression_mode,
            self.quality
        );

        Ok(())
    }

    fn encode_output(
        &self,
        img: &DynamicImage,
        output_path: &Path,
        mode: &CompressionMode,
    ) -> Result<()> {
        match self.output_format {
            OutputFormat::Webp => {
                let encoder = Encoder::from_image(img)
                    .map_err(|e| anyhow::anyhow!("Failed to create encoder: {}", e))?;
                let quality = if self.ultra_fast && self.quality > 85.0 {
                    85.0
                } else {
                    self.quality
                };
                let webp_data = match mode {
                    CompressionMode::Lossless => encoder.encode_lossless(),
                    CompressionMode::Lossy | CompressionMode::Auto => encoder.encode(quality),
                };
                std::fs::write(output_path, &*webp_data)
                    .with_context(|| format!("Failed to save file: {}", output_path.display()))?;
                Ok(())
            }
            OutputFormat::Avif => {
                // ponytail: speed 6 chosen as batch-tool compromise; expose a
                // --avif-speed flag if encoding speed ever matters to users
                const AVIF_ENCODE_SPEED: u8 = 6;
                let file = std::fs::File::create(output_path)
                    .with_context(|| format!("Failed to create file: {}", output_path.display()))?;
                let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
                    file,
                    AVIF_ENCODE_SPEED,
                    self.quality as u8,
                );
                img.write_with_encoder(encoder)
                    .with_context(|| format!("Failed to encode AVIF: {}", output_path.display()))
            }
        }
    }

    fn convert_auto_fast(
        &self,
        img: &DynamicImage,
        output_path: &Path,
        input_path: &Path,
    ) -> Result<()> {
        // Smart strategy selection: automatically choose compression mode based on image characteristics
        let should_use_lossless = self.should_use_lossless_fast(img, input_path);

        let mode = if should_use_lossless {
            CompressionMode::Lossless
        } else {
            CompressionMode::Lossy
        };
        self.encode_output(img, output_path, &mode)
    }

    fn should_use_lossless_fast(&self, img: &DynamicImage, input_path: &Path) -> bool {
        // Enhanced decision algorithm with content analysis
        let extension = input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();

        // Quick decision based on file extension
        match extension.as_str() {
            "png" | "gif" => true, // Likely have transparency or few colors
            "jpg" | "jpeg" => {
                // For JPEG, analyze image characteristics
                let (width, height) = img.dimensions();
                let total_pixels = width as u64 * height as u64;

                // Use lossless for small JPEG images (likely screenshots/graphics)
                if total_pixels < 50000 {
                    return true;
                }

                // Analyze color complexity for larger images
                self.analyze_color_complexity(img)
            }
            _ => {
                // For other formats, use comprehensive analysis
                let (width, height) = img.dimensions();
                let total_pixels = width as u64 * height as u64;

                if total_pixels < 50000 {
                    true // Small images use lossless
                } else {
                    self.analyze_color_complexity(img)
                }
            }
        }
    }

    /// Analyze color complexity to determine optimal compression mode
    fn analyze_color_complexity(&self, img: &DynamicImage) -> bool {
        // Sample pixels to estimate color complexity
        let (width, height) = img.dimensions();
        let sample_size = 100.min(width * height / 4); // Sample up to 100 pixels
        let step_x = (width / 10).max(1);
        let step_y = (height / 10).max(1);

        let mut unique_colors = std::collections::HashSet::new();
        let mut has_transparency = false;

        // Sample pixels across the image
        for y in (0..height).step_by(step_y as usize) {
            for x in (0..width).step_by(step_x as usize) {
                if unique_colors.len() > sample_size as usize {
                    break;
                }

                let pixel = img.get_pixel(x, y);
                let rgba = pixel.0;

                // Check for transparency
                if rgba.len() > 3 && rgba[3] < 255 {
                    has_transparency = true;
                }

                // Store RGB values (ignore alpha for color counting)
                unique_colors.insert((rgba[0], rgba[1], rgba[2]));
            }
        }

        // Decision logic:
        // - Use lossless if transparency detected
        // - Use lossless if low color count (graphics/logos)
        // - Use lossy for photographic content (high color count)
        has_transparency || unique_colors.len() < 64
    }

    /// Validate and potentially resize image to fit WebP constraints
    /// Returns None if no resizing is needed, Some(resized_image) if resizing was performed
    fn validate_and_resize_image(&self, img: &DynamicImage) -> Result<Option<DynamicImage>> {
        let (width, height) = img.dimensions();

        // WebP maximum dimensions are 16383x16383
        const MAX_WEBP_DIMENSION: u32 = 16383;

        if width == 0 || height == 0 {
            return Err(anyhow::anyhow!(
                "Invalid image dimensions: {}x{} (zero dimensions)",
                width,
                height
            ));
        }

        if width <= MAX_WEBP_DIMENSION && height <= MAX_WEBP_DIMENSION {
            // Image is within limits, no cloning needed
            return Ok(None);
        }

        // Image is too large, resize it to fit within WebP limits
        let scale_factor = (MAX_WEBP_DIMENSION as f64 / width.max(height) as f64).min(1.0);
        let new_width = (width as f64 * scale_factor) as u32;
        let new_height = (height as f64 * scale_factor) as u32;

        log::warn!(
            "Resizing image from {width}x{height} to {new_width}x{new_height} to fit WebP limits"
        );

        Ok(Some(img.resize(
            new_width,
            new_height,
            image::imageops::FilterType::Lanczos3,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_never_exceeds_original() {
        for mode in [
            CompressionMode::Lossless,
            CompressionMode::Lossy,
            CompressionMode::Auto,
        ] {
            for format in [OutputFormat::Webp, OutputFormat::Avif] {
                for quality in [0, 50, 80, 95] {
                    let estimate = estimate_output_size(1000, &mode, quality, &format);
                    assert!(
                        estimate <= 1000,
                        "{mode:?}/{format:?} q{quality}: {estimate}"
                    );
                }
            }
        }
    }

    #[test]
    fn avif_estimate_is_below_webp_estimate() {
        let webp = estimate_output_size(1000, &CompressionMode::Lossy, 80, &OutputFormat::Webp);
        let avif = estimate_output_size(1000, &CompressionMode::Lossy, 80, &OutputFormat::Avif);
        assert!(avif < webp);
    }

    #[test]
    fn encodes_real_avif_file() {
        use image::RgbImage;

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.avif");
        RgbImage::from_pixel(16, 16, image::Rgb([200, 100, 50]))
            .save(&input)
            .unwrap();

        let converter = ImageConverter::new_with_dry_run(
            80,
            &CompressionMode::Lossy,
            &OutputFormat::Avif,
            false,
        );
        converter.convert_to_webp(&input, &output).unwrap();

        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.len() > 12, "avif output too small");
        assert_eq!(&bytes[4..8], b"ftyp", "avif must start with an ftyp box");
        assert!(
            &bytes[8..12] == b"avif" || &bytes[8..12] == b"avis",
            "avif brand missing"
        );
    }
}
