use crate::config::ColorScheme;
use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct SystemdUnit {
    pub name: String,
    pub description: String,
    pub load_state: UnitLoadState,
    pub active_state: UnitActiveState,
    pub sub_state: String,
    pub follows: String,
    pub path: String,
}

impl SystemdUnit {
    pub fn active_glyph(&self) -> &'static str {
        match self.active_state {
            UnitActiveState::Active => "+",
            UnitActiveState::Activating => "~",
            UnitActiveState::Deactivating => "#",
            UnitActiveState::Failed => "X",
            UnitActiveState::Inactive | UnitActiveState::Unknown => " ",
        }
    }

    pub fn status_color(&self, colors: &ColorScheme) -> Color {
        match self.active_state {
            UnitActiveState::Active => colors.ok,
            UnitActiveState::Activating | UnitActiveState::Deactivating => colors.warning,
            UnitActiveState::Failed => colors.error,
            UnitActiveState::Inactive | UnitActiveState::Unknown => colors.inactive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitLoadState {
    Loaded,
    NotFound,
    BadSetting,
    Masked,
    Unknown,
}

impl UnitLoadState {
    pub fn from_raw(value: &str) -> Self {
        match value {
            "loaded" => Self::Loaded,
            "not-found" => Self::NotFound,
            "bad-setting" => Self::BadSetting,
            "masked" => Self::Masked,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitActiveState {
    Active,
    Inactive,
    Activating,
    Deactivating,
    Failed,
    Unknown,
}

impl UnitActiveState {
    pub fn from_raw(value: &str) -> Self {
        match value {
            "active" => Self::Active,
            "inactive" => Self::Inactive,
            "activating" => Self::Activating,
            "deactivating" => Self::Deactivating,
            "failed" => Self::Failed,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Activating => "activating",
            Self::Deactivating => "deactivating",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}
