use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView};
use std::path::Path;
use webp::{Encoder, WebPMemory};

use crate::CompressionMode;

pub struct ImageConverter {
    quality: f32,
    mode: CompressionMode,
    // Performance optimization: pre-calculated encoding speed
    #[allow(dead_code)]
    encoding_speed: i32,
    // Ultra-fast mode for maximum performance
    ultra_fast: bool,
}

impl ImageConverter {
    pub fn new(quality: u8, mode: &CompressionMode) -> Self {
        Self {
            quality: quality as f32,
            mode: mode.clone(),
            // Use fastest encoding for maximum performance (0=fastest, 6=slowest)
            encoding_speed: 0, // Ultra-fast encoding
            ultra_fast: true,
        }
    }

    pub fn new_with_speed(quality: u8, mode: &CompressionMode, ultra_fast: bool) -> Self {
        Self {
            quality: quality as f32,
            mode: mode.clone(),
            encoding_speed: if ultra_fast { 0 } else { 1 },
            ultra_fast,
        }
    }

    pub fn convert_to_webp(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        // Performance: Read image with optimized buffer size
        let img = image::open(input_path)
            .with_context(|| format!("Failed to read image: {}", input_path.display()))?;

        // Validate and potentially resize image to fit WebP constraints
        let validated_img = self.validate_and_resize_image(&img)
            .with_context(|| format!("Image validation failed for: {}", input_path.display()))?;

        // Choose conversion strategy based on mode
        match self.mode {
            CompressionMode::Lossless => self.convert_lossless_fast(&validated_img, output_path),
            CompressionMode::Lossy => self.convert_lossy_fast(&validated_img, output_path),
            CompressionMode::Auto => self.convert_auto_fast(&validated_img, output_path, input_path),
        }
    }

    fn convert_lossless_fast(&self, img: &DynamicImage, output_path: &Path) -> Result<()> {
        let encoder = Encoder::from_image(img)
            .map_err(|e| anyhow::anyhow!("Failed to create encoder: {}", e))?;
        
        // Performance: Use faster encoding method with error handling
        let webp_data = encoder.encode_lossless();
        self.save_webp_data_fast(&webp_data, output_path)
    }

    fn convert_lossy_fast(&self, img: &DynamicImage, output_path: &Path) -> Result<()> {
        let encoder = Encoder::from_image(img)
            .map_err(|e| anyhow::anyhow!("Failed to create encoder: {}", e))?;
        
        // Performance: Use ultra-fast encoding with optimized quality
        let quality = if self.ultra_fast && self.quality > 85.0 {
            // For ultra-fast mode, cap quality to balance speed vs size
            85.0
        } else {
            self.quality
        };
        
        let webp_data = encoder.encode(quality);
        self.save_webp_data_fast(&webp_data, output_path)
    }

    fn convert_auto_fast(&self, img: &DynamicImage, output_path: &Path, input_path: &Path) -> Result<()> {
        // Smart strategy selection: automatically choose compression mode based on image characteristics
        let should_use_lossless = self.should_use_lossless_fast(img, input_path);
        
        if should_use_lossless {
            self.convert_lossless_fast(img, output_path)
        } else {
            self.convert_lossy_fast(img, output_path)
        }
    }

    fn should_use_lossless_fast(&self, img: &DynamicImage, input_path: &Path) -> bool {
        // Fast decision algorithm - simplified logic for performance
        let extension = input_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();
        
        // Performance: Quick decision based on file extension only
        match extension.as_str() {
            "png" | "gif" => true,  // Likely have transparency or few colors
            "jpg" | "jpeg" => false, // Photo content, use lossy
            _ => {
                // For other formats, quick size-based decision
                let (width, height) = img.dimensions();
                let total_pixels = width as u64 * height as u64;
                total_pixels < 50000 // Small images use lossless
            }
        }
    }

    fn save_webp_data_fast(&self, webp_data: &WebPMemory, output_path: &Path) -> Result<()> {
        // Performance: Use optimized file writing with correct dereferencing
        std::fs::write(output_path, &**webp_data)
            .with_context(|| format!("Failed to save WebP file: {}", output_path.display()))?;
        Ok(())
    }

    // Legacy methods for compatibility
    #[allow(dead_code)]
    fn save_webp_data(&self, webp_data: &WebPMemory, output_path: &Path) -> Result<()> {
        std::fs::write(output_path, &**webp_data)
            .with_context(|| format!("Failed to save WebP file: {}", output_path.display()))?;
        Ok(())
    }

    /// Get supported input formats
    #[allow(dead_code)]
    pub fn supported_formats() -> Vec<&'static str> {
        vec!["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"]
    }

    /// Check if file is a supported format
    #[allow(dead_code)]
    pub fn is_supported_format(path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let ext_lower = extension.to_lowercase();
            Self::supported_formats().contains(&ext_lower.as_str())
        } else {
            false
        }
    }

    /// Estimate compression ratio
    #[allow(dead_code)]
    pub fn estimate_compression_ratio(&self, format: &str, mode: &CompressionMode) -> f64 {
        match (format.to_lowercase().as_str(), mode) {
            ("jpg" | "jpeg", CompressionMode::Lossy) => 0.7,    // JPG to WebP lossy ~70%
            ("jpg" | "jpeg", CompressionMode::Lossless) => 0.9, // JPG to WebP lossless ~90%
            ("png", CompressionMode::Lossy) => 0.4,             // PNG to WebP lossy ~40%
            ("png", CompressionMode::Lossless) => 0.6,          // PNG to WebP lossless ~60%
            ("gif", _) => 0.5,                                  // GIF to WebP ~50%
            ("bmp", _) => 0.2,                                  // BMP to WebP ~20%
            ("tiff", _) => 0.3,                                 // TIFF to WebP ~30%
            ("webp", _) => 1.0,                                 // WebP to WebP ~unchanged
            _ => 0.5,                                           // Default 50%
        }
    }

    /// Validate and potentially resize image to fit WebP constraints
    fn validate_and_resize_image(&self, img: &DynamicImage) -> Result<DynamicImage> {
        let (width, height) = img.dimensions();
        
        // WebP maximum dimensions are 16383x16383
        const MAX_WEBP_DIMENSION: u32 = 16383;
        
        if width == 0 || height == 0 {
            return Err(anyhow::anyhow!("Invalid image dimensions: {}x{} (zero dimensions)", width, height));
        }
        
        if width <= MAX_WEBP_DIMENSION && height <= MAX_WEBP_DIMENSION {
            // Image is within limits, return as-is
            return Ok(img.clone());
        }
        
        // Image is too large, resize it to fit within WebP limits
        let scale_factor = (MAX_WEBP_DIMENSION as f64 / width.max(height) as f64).min(1.0);
        let new_width = (width as f64 * scale_factor) as u32;
        let new_height = (height as f64 * scale_factor) as u32;
        
        log::warn!("Resizing image from {}x{} to {}x{} to fit WebP limits", 
                   width, height, new_width, new_height);
        
        Ok(img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3))
    }
}
