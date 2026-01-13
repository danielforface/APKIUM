//! Theme Engine
//! 
//! Provides theme definitions and customization for the IDE.

use serde::{Deserialize, Serialize};

/// RGB Color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Self::rgb(r, g, b))
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Self::rgba(r, g, b, a))
        } else {
            None
        }
    }

    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    pub fn with_alpha(&self, alpha: u8) -> Self {
        Self { a: alpha, ..*self }
    }

    pub fn brighter(&self, factor: f32) -> Self {
        let factor = 1.0 + factor;
        Self {
            r: (self.r as f32 * factor).min(255.0) as u8,
            g: (self.g as f32 * factor).min(255.0) as u8,
            b: (self.b as f32 * factor).min(255.0) as u8,
            a: self.a,
        }
    }

    pub fn darker(&self, factor: f32) -> Self {
        let factor = 1.0 - factor;
        Self {
            r: (self.r as f32 * factor) as u8,
            g: (self.g as f32 * factor) as u8,
            b: (self.b as f32 * factor) as u8,
            a: self.a,
        }
    }
}

/// Syntax highlighting colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxColors {
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub function: Color,
    pub type_name: Color,
    pub variable: Color,
    pub operator: Color,
    pub attribute: Color,
    pub macro_name: Color,
}

/// UI Theme definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    
    // Background colors
    pub background: Color,
    pub surface: Color,
    pub surface_elevated: Color,
    pub surface_glass: Color,
    
    // Primary accent colors
    pub primary: Color,
    pub primary_hover: Color,
    pub secondary: Color,
    pub accent: Color,
    
    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    
    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    
    // Border colors
    pub border: Color,
    pub border_focus: Color,
    
    // Syntax highlighting
    pub syntax: SyntaxColors,
    
    // UI settings
    pub glassmorphism_enabled: bool,
    pub animation_speed: f32,
    pub border_radius: u32,
}

impl Theme {
    /// Dark Neon theme - the default 2026 aesthetic
    pub fn dark_neon() -> Self {
        Self {
            name: "Dark Neon".into(),
            
            background: Color::from_hex("#0a0a0f").unwrap(),
            surface: Color::from_hex("#12121a").unwrap(),
            surface_elevated: Color::from_hex("#1a1a24").unwrap(),
            surface_glass: Color::from_hex("#1a1a2480").unwrap(),
            
            primary: Color::from_hex("#6366f1").unwrap(),
            primary_hover: Color::from_hex("#818cf8").unwrap(),
            secondary: Color::from_hex("#22d3ee").unwrap(),
            accent: Color::from_hex("#f472b6").unwrap(),
            
            success: Color::from_hex("#34d399").unwrap(),
            warning: Color::from_hex("#fbbf24").unwrap(),
            error: Color::from_hex("#f87171").unwrap(),
            info: Color::from_hex("#60a5fa").unwrap(),
            
            text_primary: Color::from_hex("#f8fafc").unwrap(),
            text_secondary: Color::from_hex("#94a3b8").unwrap(),
            text_muted: Color::from_hex("#64748b").unwrap(),
            
            border: Color::from_hex("#2d2d3a").unwrap(),
            border_focus: Color::from_hex("#6366f1").unwrap(),
            
            syntax: SyntaxColors {
                keyword: Color::from_hex("#c792ea").unwrap(),
                string: Color::from_hex("#c3e88d").unwrap(),
                number: Color::from_hex("#f78c6c").unwrap(),
                comment: Color::from_hex("#546e7a").unwrap(),
                function: Color::from_hex("#82aaff").unwrap(),
                type_name: Color::from_hex("#ffcb6b").unwrap(),
                variable: Color::from_hex("#f8fafc").unwrap(),
                operator: Color::from_hex("#89ddff").unwrap(),
                attribute: Color::from_hex("#c792ea").unwrap(),
                macro_name: Color::from_hex("#82aaff").unwrap(),
            },
            
            glassmorphism_enabled: true,
            animation_speed: 0.8,
            border_radius: 8,
        }
    }

    /// Light Minimal theme
    pub fn light_minimal() -> Self {
        Self {
            name: "Light Minimal".into(),
            
            background: Color::from_hex("#ffffff").unwrap(),
            surface: Color::from_hex("#f8fafc").unwrap(),
            surface_elevated: Color::from_hex("#f1f5f9").unwrap(),
            surface_glass: Color::from_hex("#f8fafc80").unwrap(),
            
            primary: Color::from_hex("#4f46e5").unwrap(),
            primary_hover: Color::from_hex("#6366f1").unwrap(),
            secondary: Color::from_hex("#0891b2").unwrap(),
            accent: Color::from_hex("#db2777").unwrap(),
            
            success: Color::from_hex("#059669").unwrap(),
            warning: Color::from_hex("#d97706").unwrap(),
            error: Color::from_hex("#dc2626").unwrap(),
            info: Color::from_hex("#2563eb").unwrap(),
            
            text_primary: Color::from_hex("#0f172a").unwrap(),
            text_secondary: Color::from_hex("#475569").unwrap(),
            text_muted: Color::from_hex("#94a3b8").unwrap(),
            
            border: Color::from_hex("#e2e8f0").unwrap(),
            border_focus: Color::from_hex("#4f46e5").unwrap(),
            
            syntax: SyntaxColors {
                keyword: Color::from_hex("#7c3aed").unwrap(),
                string: Color::from_hex("#059669").unwrap(),
                number: Color::from_hex("#ea580c").unwrap(),
                comment: Color::from_hex("#94a3b8").unwrap(),
                function: Color::from_hex("#2563eb").unwrap(),
                type_name: Color::from_hex("#ca8a04").unwrap(),
                variable: Color::from_hex("#0f172a").unwrap(),
                operator: Color::from_hex("#0891b2").unwrap(),
                attribute: Color::from_hex("#7c3aed").unwrap(),
                macro_name: Color::from_hex("#2563eb").unwrap(),
            },
            
            glassmorphism_enabled: false,
            animation_speed: 0.8,
            border_radius: 8,
        }
    }

    /// Midnight theme - deep dark
    pub fn midnight() -> Self {
        Self {
            name: "Midnight".into(),
            
            background: Color::from_hex("#000000").unwrap(),
            surface: Color::from_hex("#0a0a0a").unwrap(),
            surface_elevated: Color::from_hex("#141414").unwrap(),
            surface_glass: Color::from_hex("#14141480").unwrap(),
            
            primary: Color::from_hex("#a855f7").unwrap(),
            primary_hover: Color::from_hex("#c084fc").unwrap(),
            secondary: Color::from_hex("#06b6d4").unwrap(),
            accent: Color::from_hex("#ec4899").unwrap(),
            
            success: Color::from_hex("#22c55e").unwrap(),
            warning: Color::from_hex("#eab308").unwrap(),
            error: Color::from_hex("#ef4444").unwrap(),
            info: Color::from_hex("#3b82f6").unwrap(),
            
            text_primary: Color::from_hex("#ffffff").unwrap(),
            text_secondary: Color::from_hex("#a1a1aa").unwrap(),
            text_muted: Color::from_hex("#71717a").unwrap(),
            
            border: Color::from_hex("#27272a").unwrap(),
            border_focus: Color::from_hex("#a855f7").unwrap(),
            
            syntax: SyntaxColors {
                keyword: Color::from_hex("#c084fc").unwrap(),
                string: Color::from_hex("#86efac").unwrap(),
                number: Color::from_hex("#fdba74").unwrap(),
                comment: Color::from_hex("#71717a").unwrap(),
                function: Color::from_hex("#93c5fd").unwrap(),
                type_name: Color::from_hex("#fde047").unwrap(),
                variable: Color::from_hex("#ffffff").unwrap(),
                operator: Color::from_hex("#67e8f9").unwrap(),
                attribute: Color::from_hex("#c084fc").unwrap(),
                macro_name: Color::from_hex("#93c5fd").unwrap(),
            },
            
            glassmorphism_enabled: true,
            animation_speed: 0.8,
            border_radius: 8,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark_neon()
    }
}

/// Theme manager for the application
pub struct ThemeManager {
    current: Theme,
    available: Vec<Theme>,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current: Theme::dark_neon(),
            available: vec![
                Theme::dark_neon(),
                Theme::light_minimal(),
                Theme::midnight(),
            ],
        }
    }

    pub fn current(&self) -> &Theme {
        &self.current
    }

    pub fn set_theme(&mut self, name: &str) -> bool {
        if let Some(theme) = self.available.iter().find(|t| t.name == name) {
            self.current = theme.clone();
            true
        } else {
            false
        }
    }

    pub fn available_themes(&self) -> &[Theme] {
        &self.available
    }

    pub fn add_custom_theme(&mut self, theme: Theme) {
        self.available.push(theme);
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#ff0000").unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
    }

    #[test]
    fn test_color_to_hex() {
        let color = Color::rgb(255, 0, 0);
        assert_eq!(color.to_hex(), "#ff0000");
    }

    #[test]
    fn test_theme_manager() {
        let mut manager = ThemeManager::new();
        assert_eq!(manager.current().name, "Dark Neon");
        
        manager.set_theme("Light Minimal");
        assert_eq!(manager.current().name, "Light Minimal");
    }
}
