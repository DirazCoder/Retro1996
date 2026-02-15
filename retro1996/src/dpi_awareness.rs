use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DpiManager {
    pub current_dpi: u32,
    pub scale_factor: f32,
    pub base_dpi: u32,
}

impl DpiManager {
    pub fn new() -> Self {
        let base_dpi = 96; // Standard Windows DPI
        // For now, we'll use a default DPI - in a real implementation we'd detect the actual system DPI
        let current_dpi = base_dpi; // Default to standard DPI
        let scale_factor = current_dpi as f32 / base_dpi as f32;

        DpiManager {
            current_dpi,
            scale_factor,
            base_dpi,
        }
    }

    pub fn initialize_dpi_awareness() -> Result<(), String> {
        // For cross-platform compatibility, we'll just return Ok
        // In a real implementation, this would set platform-specific DPI awareness
        Ok(())
    }

    pub fn get_system_dpi() -> Option<u32> {
        // In a real implementation, this would detect the actual system DPI
        // For now, we'll return a default value
        Some(96) // Default DPI
    }

    pub fn update_dpi(&mut self, new_dpi: u32) {
        self.current_dpi = new_dpi;
        self.scale_factor = new_dpi as f32 / self.base_dpi as f32;
    }

    pub fn scale_value(&self, value: f32) -> f32 {
        value * self.scale_factor
    }

    pub fn scale_point(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.scale_factor, y * self.scale_factor)
    }

    pub fn scale_rect(&self, x: f32, y: f32, width: f32, height: f32) -> (f32, f32, f32, f32) {
        (
            x * self.scale_factor,
            y * self.scale_factor,
            width * self.scale_factor,
            height * self.scale_factor,
        )
    }

    pub fn unscale_value(&self, value: f32) -> f32 {
        value / self.scale_factor
    }

    pub fn get_scaled_font_size(&self, base_size: f32) -> f32 {
        self.scale_value(base_size)
    }

    pub fn get_scaled_image_size(&self, width: u32, height: u32) -> (u32, u32) {
        (
            (width as f32 * self.scale_factor) as u32,
            (height as f32 * self.scale_factor) as u32,
        )
    }

    pub fn get_scaled_ui_element_size(&self, base_width: f32, base_height: f32) -> (f32, f32) {
        (
            self.scale_value(base_width),
            self.scale_value(base_height),
        )
    }

    pub fn get_appropriate_icon_size(&self) -> u32 {
        // Return appropriate icon size based on DPI
        match self.current_dpi {
            dpi if dpi <= 96 => 16,   // 100% scale
            dpi if dpi <= 120 => 24,  // 125% scale
            dpi if dpi <= 144 => 32,  // 150% scale
            dpi if dpi <= 192 => 48,  // 200% scale
            _ => 64,                  // Higher scale
        }
    }

    pub fn get_scaled_window_size(&self, base_width: u32, base_height: u32) -> (u32, u32) {
        (
            (base_width as f32 * self.scale_factor) as u32,
            (base_height as f32 * self.scale_factor) as u32,
        )
    }

    pub fn get_scaled_margin(&self, base_margin: f32) -> f32 {
        self.scale_value(base_margin)
    }

    pub fn get_scaled_padding(&self, base_padding: f32) -> f32 {
        self.scale_value(base_padding)
    }

    pub fn get_scaled_border_width(&self, base_width: f32) -> f32 {
        // For 1996-style UI, we want to keep borders at least 1 pixel even when scaled
        let scaled = self.scale_value(base_width);
        scaled.max(1.0)
    }

    pub fn should_use_high_dpi_icons(&self) -> bool {
        self.scale_factor > 1.0
    }

    pub fn get_dpi_scale_info(&self) -> DpiScaleInfo {
        DpiScaleInfo {
            dpi: self.current_dpi,
            scale_factor: self.scale_factor,
            scale_percentage: (self.scale_factor * 100.0) as u32,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DpiScaleInfo {
    pub dpi: u32,
    pub scale_factor: f32,
    pub scale_percentage: u32,
}

// Helper functions for common DPI scaling operations
pub fn scale_font_size(base_size: f32, scale_factor: f32) -> f32 {
    let scaled = base_size * scale_factor;
    // Ensure font size is reasonable for 1996-era rendering
    scaled.clamp(8.0, 72.0) // Min 8pt, Max 72pt (reasonable for 1996)
}

pub fn scale_image_dimension(base_dim: u32, scale_factor: f32) -> u32 {
    let scaled = (base_dim as f32) * scale_factor;
    scaled.round() as u32
}

pub fn scale_css_pixel(pixel_value: f32, scale_factor: f32) -> f32 {
    // For 1996-era browsers, we need to be careful about scaling
    // Some elements should remain crisp at 1px even on high-DPI displays
    if pixel_value == 1.0 {
        // For 1px borders, we might want to keep them at 1px for clarity
        // But for other elements, we scale normally
        if scale_factor >= 2.0 {
            // At 200% scaling and above, consider allowing 1px to become 2px for visibility
            (pixel_value * scale_factor).max(1.0)
        } else {
            pixel_value
        }
    } else {
        pixel_value * scale_factor
    }
}

pub fn scale_legacy_ui_element(base_size: f32, scale_factor: f32) -> f32 {
    // Apply scaling but preserve the authentic 1996 look
    let scaled = base_size * scale_factor;

    // Round to nearest integer to maintain crispness on high-DPI displays
    scaled.round()
}

// Legacy 1996-compatible scaling factors
pub fn get_legacy_scale_factor(dpi: u32) -> f32 {
    match dpi {
        96 => 1.0,    // 100% - Standard DPI
        120 => 1.25,  // 125% - Common high-DPI setting
        144 => 1.5,   // 150% - Higher resolution
        192 => 2.0,   // 200% - High-DPI displays
        _ => dpi as f32 / 96.0, // Calculate for other DPI values
    }
}

// Function to determine if we're on a high-DPI system
pub fn is_high_dpi_system() -> bool {
    match DpiManager::get_system_dpi() {
        Some(dpi) => dpi > 96,
        None => false,
    }
}

// Function to get recommended UI scaling for 1996 aesthetics on high-DPI displays
pub fn get_recommended_legacy_scaling(dpi: u32) -> f32 {
    // For 1996 browsers, we want to maintain the authentic look while ensuring readability
    match dpi {
        // Standard DPI - no scaling needed
        d if d <= 96 => 1.0,
        // For moderate DPI increases, we might scale slightly to maintain readability
        d if d <= 120 => 1.0, // Keep 1:1 for authentic look
        // For higher DPI, we might need to scale to maintain usability
        d if d <= 144 => 1.0, // Still keep 1:1 for authentic look
        d if d <= 192 => 1.2, // At 200% DPI, slightly scale UI elements to maintain usability
        d if d <= 240 => 1.5, // At 250% DPI, increase scaling
        _ => 2.0, // For very high DPI, use 2x scaling
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpi_manager_initialization() {
        let dpi_manager = DpiManager::new();
        assert!(dpi_manager.current_dpi > 0);
        assert!(dpi_manager.scale_factor >= 1.0);
    }

    #[test]
    fn test_scaling_functions() {
        let dpi_manager = DpiManager::new();

        // Test basic scaling
        assert_eq!(dpi_manager.scale_value(10.0), 10.0 * dpi_manager.scale_factor);

        // Test font scaling
        let scaled_font = scale_font_size(12.0, 1.5);
        assert_eq!(scaled_font, 18.0);

        // Test image scaling
        let scaled_w = scale_image_dimension(100, 1.5);
        assert_eq!(scaled_w, 150);
    }

    #[test]
    fn test_legacy_scale_factors() {
        assert_eq!(get_legacy_scale_factor(96), 1.0);
        assert_eq!(get_legacy_scale_factor(120), 1.25);
        assert_eq!(get_legacy_scale_factor(144), 1.5);
        assert_eq!(get_legacy_scale_factor(192), 2.0);
    }

    #[test]
    fn test_css_pixel_scaling() {
        // 1px should remain 1px at standard DPI
        assert_eq!(scale_css_pixel(1.0, 1.0), 1.0);

        // At high DPI, 1px might scale differently depending on implementation
        let high_dpi_result = scale_css_pixel(1.0, 2.0);
        assert!(high_dpi_result >= 1.0);
    }
}