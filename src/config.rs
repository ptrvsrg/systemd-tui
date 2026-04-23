use crate::bus::ConnectionConfig;
use ratatui::style::Color;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub refresh_interval: Duration,
    pub colors: ColorScheme,
    pub connection: ConnectionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval: Duration::from_secs(2),
            colors: ColorScheme::default(),
            connection: ConnectionConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub ok: Color,
    pub warning: Color,
    pub error: Color,
    pub inactive: Color,
    pub header_bg: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            ok: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            inactive: Color::DarkGray,
            header_bg: Color::Blue,
        }
    }
}
