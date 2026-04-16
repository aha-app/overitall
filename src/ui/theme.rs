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

const LIGHT_PROCESS_PALETTE: &[Color] = &[
    Color::Blue,
    Color::Magenta,
    Color::Rgb(0, 120, 0),
    Color::Rgb(120, 80, 0),
    Color::Rgb(0, 100, 140),
    Color::Rgb(140, 0, 140),
    Color::Rgb(0, 100, 100),
    Color::Rgb(160, 0, 80),
    Color::Rgb(80, 80, 140),
    Color::Rgb(100, 60, 100),
];

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub footer_bg: Color,
    pub footer_fg: Color,
    pub muted: Color,
    pub accent: Color,
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
            selection_bg: Color::Rgb(30, 50, 70),
            selection_fg: Color::White,
            process_palette: DARK_PROCESS_PALETTE,
            fallback_process: Color::White,
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            footer_bg: Color::Rgb(220, 220, 220),
            footer_fg: Color::Black,
            muted: Color::DarkGray,
            accent: Color::Rgb(140, 90, 0),
            selection_bg: Color::Rgb(180, 200, 220),
            selection_fg: Color::Black,
            process_palette: LIGHT_PROCESS_PALETTE,
            fallback_process: Color::Black,
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
    fn light_theme_uses_dark_first_palette_color() {
        assert_eq!(Theme::light().process_palette[0], Color::Blue);
    }
}
