use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

const DARK_PROCESS_PALETTE: &[Color] = &[
    Color::Green,
    Color::Yellow,
    Color::Cyan,
    Color::Blue,
    Color::Magenta,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightCyan,
    Color::LightBlue,
    Color::LightMagenta,
];

// Gruvbox-light "faded" accents — chosen for legibility on warm cream backgrounds.
const LIGHT_PROCESS_PALETTE: &[Color] = &[
    Color::Rgb(0x07, 0x66, 0x78),  // blue
    Color::Rgb(0x8f, 0x3f, 0x71),  // purple
    Color::Rgb(0x79, 0x74, 0x0e),  // green
    Color::Rgb(0xaf, 0x3a, 0x03),  // orange
    Color::Rgb(0x42, 0x7b, 0x58),  // aqua
    Color::Rgb(0xb5, 0x76, 0x14),  // yellow
    Color::Rgb(0x9d, 0x00, 0x06),  // red
    Color::Rgb(0x66, 0x5c, 0x54),  // brown / fg3
];

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub footer_bg: Color,
    pub footer_fg: Color,
    pub muted: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub info: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub process_palette: &'static [Color],
    pub fallback_process: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            footer_bg: Color::Rgb(40, 40, 40),
            footer_fg: Color::Reset,
            muted: Color::Gray,
            accent: Color::Yellow,
            success: Color::Green,
            error: Color::Red,
            info: Color::Yellow,
            selection_bg: Color::Rgb(30, 50, 70),
            selection_fg: Color::White,
            process_palette: DARK_PROCESS_PALETTE,
            fallback_process: Color::White,
        }
    }

    /// Gruvbox-light inspired palette. Footer uses bg2 / fg1 for clear contrast
    /// against typical cream terminal backgrounds (gruvbox bg0 ≈ #fbf1c7).
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            footer_bg: Color::Rgb(0xd5, 0xc4, 0xa1),       // bg2
            footer_fg: Color::Rgb(0x3c, 0x38, 0x36),       // fg1
            muted: Color::Rgb(0x7c, 0x6f, 0x64),           // fg4
            accent: Color::Rgb(0xb5, 0x76, 0x14),          // faded yellow
            success: Color::Rgb(0x79, 0x74, 0x0e),         // faded green
            error: Color::Rgb(0x9d, 0x00, 0x06),           // faded red
            info: Color::Rgb(0xb5, 0x76, 0x14),            // faded yellow
            selection_bg: Color::Rgb(0xbd, 0xae, 0x93),    // bg3
            selection_fg: Color::Rgb(0x28, 0x28, 0x28),    // fg0
            process_palette: LIGHT_PROCESS_PALETTE,
            fallback_process: Color::Rgb(0x3c, 0x38, 0x36),
        }
    }

    pub fn from_config(value: Option<&str>) -> Self {
        match value.map(|s| s.to_lowercase()).as_deref() {
            Some("light") => Self::light(),
            _ => Self::dark(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_defaults_to_dark() {
        assert_eq!(Theme::from_config(None).mode, ThemeMode::Dark);
    }

    #[test]
    fn from_config_parses_light() {
        assert_eq!(Theme::from_config(Some("light")).mode, ThemeMode::Light);
    }

    #[test]
    fn from_config_is_case_insensitive() {
        assert_eq!(Theme::from_config(Some("LIGHT")).mode, ThemeMode::Light);
        assert_eq!(Theme::from_config(Some("Dark")).mode, ThemeMode::Dark);
    }

    #[test]
    fn from_config_unknown_falls_back_to_dark() {
        assert_eq!(Theme::from_config(Some("solarized")).mode, ThemeMode::Dark);
    }

    #[test]
    fn light_palette_first_color_is_gruvbox_blue() {
        assert_eq!(
            Theme::light().process_palette[0],
            Color::Rgb(0x07, 0x66, 0x78)
        );
    }

    #[test]
    fn light_theme_status_colors_are_faded() {
        let t = Theme::light();
        assert_eq!(t.success, Color::Rgb(0x79, 0x74, 0x0e));
        assert_eq!(t.error, Color::Rgb(0x9d, 0x00, 0x06));
    }
}
