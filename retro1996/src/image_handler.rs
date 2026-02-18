use std::collections::HashMap;
use image::{ImageFormat, RgbaImage, GenericImageView};
use crate::animation::{Animation, AnimationType, AnimationFrame};

#[derive(Debug, Clone)]
pub struct ImageCache {
    cache: HashMap<String, CachedImage>,
}

#[derive(Debug, Clone)]
pub enum CachedImage {
    Static(RgbaImage),
    Animated(Animation),
}

impl ImageCache {
    pub fn new() -> Self {
        ImageCache {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&CachedImage> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: String, image: CachedImage) {
        self.cache.insert(key, image);
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.cache.contains_key(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<CachedImage> {
        self.cache.remove(key)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

pub struct ImageDecoder;

impl ImageDecoder {
    pub fn decode(data: &[u8]) -> Result<CachedImage, String> {
        // First, check if it's an animated GIF
        if let Ok(animation) = Animation::from_gif_data(data) {
            // This is an animated GIF
            Ok(CachedImage::Animated(animation))
        } else {
            // Try to decode as a static image
            match image::load_from_memory_with_format(data, ImageFormat::Png)
                .or_else(|_| image::load_from_memory_with_format(data, ImageFormat::Jpeg))
                .or_else(|_| image::load_from_memory_with_format(data, ImageFormat::Gif))
                .or_else(|_| image::load_from_memory_with_format(data, ImageFormat::Bmp))
            {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    Ok(CachedImage::Static(rgba))
                }
                Err(e) => Err(format!("Failed to decode image: {}", e)),
            }
        }
    }

    pub fn decode_gif(data: &[u8]) -> Result<Animation, String> {
        Animation::from_gif_data(data)
            .map_err(|e| format!("Failed to parse GIF: {:?}", e))
    }

    pub fn decode_jpeg(data: &[u8]) -> Result<RgbaImage, String> {
        image::load_from_memory_with_format(data, ImageFormat::Jpeg)
            .map(|img| img.to_rgba8())
            .map_err(|e| format!("Failed to decode JPEG: {}", e))
    }

    pub fn decode_png(data: &[u8]) -> Result<RgbaImage, String> {
        image::load_from_memory_with_format(data, ImageFormat::Png)
            .map(|img| img.to_rgba8())
            .map_err(|e| format!("Failed to decode PNG: {}", e))
    }

    pub fn decode_bmp(data: &[u8]) -> Result<RgbaImage, String> {
        image::load_from_memory_with_format(data, ImageFormat::Bmp)
            .map(|img| img.to_rgba8())
            .map_err(|e| format!("Failed to decode BMP: {}", e))
    }

    pub fn is_gif(data: &[u8]) -> bool {
        data.len() >= 6 && &data[0..6] == b"GIF87a" || &data[0..6] == b"GIF89a"
    }

    pub fn is_jpeg(data: &[u8]) -> bool {
        data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8
    }

    pub fn is_png(data: &[u8]) -> bool {
        data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n"
    }

    pub fn is_bmp(data: &[u8]) -> bool {
        data.len() >= 2 && data[0] == 0x42 && data[1] == 0x4D  // "BM"
    }

    pub fn detect_format(data: &[u8]) -> Option<ImageFormat> {
        if Self::is_gif(data) {
            Some(ImageFormat::Gif)
        } else if Self::is_jpeg(data) {
            Some(ImageFormat::Jpeg)
        } else if Self::is_png(data) {
            Some(ImageFormat::Png)
        } else if Self::is_bmp(data) {
            Some(ImageFormat::Bmp)
        } else {
            None
        }
    }
}

// GIF Decoder Implementation
pub struct GifDecoder;

impl GifDecoder {
    pub fn decode(data: &[u8]) -> Result<Animation, String> {
        // Parse the GIF data using our Animation
        Animation::from_gif_data(data)
            .map_err(|e| format!("GIF parsing error: {:?}", e))
    }

    pub fn get_frame_count(gif_data: &Animation) -> usize {
        gif_data.frames.len()
    }

    pub fn get_dimensions(gif_data: &Animation) -> (u32, u32) {
        (gif_data.width, gif_data.height)
    }

    pub fn get_loop_count(gif_data: &Animation) -> Option<u16> {
        gif_data.loop_count
    }
}

// JPEG Decoder Implementation
pub struct JpegDecoder;

impl JpegDecoder {
    pub fn decode(data: &[u8]) -> Result<RgbaImage, String> {
        image::load_from_memory_with_format(data, ImageFormat::Jpeg)
            .map(|img| img.to_rgba8())
            .map_err(|e| format!("JPEG decoding error: {}", e))
    }

    pub fn get_dimensions(data: &[u8]) -> Result<(u32, u32), String> {
        let img = image::load_from_memory_with_format(data, ImageFormat::Jpeg)
            .map_err(|e| format!("JPEG dimension extraction error: {}", e))?;
        Ok(img.dimensions())
    }
}

// PNG Decoder Implementation
pub struct PngDecoder;

impl PngDecoder {
    pub fn decode(data: &[u8]) -> Result<RgbaImage, String> {
        image::load_from_memory_with_format(data, ImageFormat::Png)
            .map(|img| img.to_rgba8())
            .map_err(|e| format!("PNG decoding error: {}", e))
    }

    pub fn get_dimensions(data: &[u8]) -> Result<(u32, u32), String> {
        let img = image::load_from_memory_with_format(data, ImageFormat::Png)
            .map_err(|e| format!("PNG dimension extraction error: {}", e))?;
        Ok(img.dimensions())
    }
}

// BMP Decoder Implementation
pub struct BmpDecoder;

impl BmpDecoder {
    pub fn decode(data: &[u8]) -> Result<RgbaImage, String> {
        image::load_from_memory_with_format(data, ImageFormat::Bmp)
            .map(|img| img.to_rgba8())
            .map_err(|e| format!("BMP decoding error: {}", e))
    }

    pub fn get_dimensions(data: &[u8]) -> Result<(u32, u32), String> {
        let img = image::load_from_memory_with_format(data, ImageFormat::Bmp)
            .map_err(|e| format!("BMP dimension extraction error: {}", e))?;
        Ok(img.dimensions())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_decoder() {
        // Test with a minimal valid PNG file (header only)
        let png_header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89";
        assert!(ImageDecoder::decode(png_header).is_err()); // Should fail because it's not a complete PNG
        
        // Test format detection
        assert_eq!(ImageDecoder::detect_format(png_header), Some(ImageFormat::Png));
    }

    #[test]
    fn test_gif_detection() {
        let gif87a = b"GIF87a\x01\x00\x01\x00\x00\x00\x00";
        let gif89a = b"GIF89a\x01\x00\x01\x00\x00\x00\x00";
        
        assert!(ImageDecoder::is_gif(gif87a));
        assert!(ImageDecoder::is_gif(gif89a));
    }

    #[test]
    fn test_jpeg_detection() {
        let jpeg = b"\xff\xd8\xff\xe0\x00\x10JFIF";
        assert!(ImageDecoder::is_jpeg(jpeg));
    }

    #[test]
    fn test_png_detection() {
        let png = b"\x89PNG\r\n\x1a\n";
        assert!(ImageDecoder::is_png(png));
    }

    #[test]
    fn test_bmp_detection() {
        let bmp = b"BM";
        assert!(ImageDecoder::is_bmp(bmp));
    }
}