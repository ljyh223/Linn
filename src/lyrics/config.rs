#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsV2Mode {
    Legacy,
    Shadow,
    Cairo,
    OpenGl,
}

impl LyricsV2Mode {
    pub fn from_environment(experimental_enabled: bool) -> Self {
        let override_value = std::env::var("LINN_LYRICS_V2").ok();
        Self::from_override(override_value.as_deref(), experimental_enabled)
    }

    fn from_override(override_value: Option<&str>, experimental_enabled: bool) -> Self {
        override_value
            .map(Self::from_value)
            .unwrap_or(if experimental_enabled {
                Self::OpenGl
            } else {
                Self::Legacy
            })
    }

    pub fn from_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "legacy" | "off" => Self::Legacy,
            "shadow" => Self::Shadow,
            "cairo" => Self::Cairo,
            "gl" | "opengl" => Self::OpenGl,
            _ => Self::Legacy,
        }
    }

    pub fn runs_v2(self) -> bool {
        self != Self::Legacy
    }

    pub fn runs_legacy(self) -> bool {
        matches!(self, Self::Legacy | Self::Shadow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_values_fail_closed_to_legacy() {
        assert_eq!(LyricsV2Mode::from_value("unknown"), LyricsV2Mode::Legacy);
    }

    #[test]
    fn accepts_documented_modes_case_insensitively() {
        assert_eq!(LyricsV2Mode::from_value("SHADOW"), LyricsV2Mode::Shadow);
        assert_eq!(LyricsV2Mode::from_value("cairo"), LyricsV2Mode::Cairo);
        assert_eq!(LyricsV2Mode::from_value("opengl"), LyricsV2Mode::OpenGl);
        assert_eq!(LyricsV2Mode::from_value("off"), LyricsV2Mode::Legacy);
    }

    #[test]
    fn user_setting_enables_gl_but_an_environment_override_wins() {
        assert_eq!(
            LyricsV2Mode::from_override(None, false),
            LyricsV2Mode::Legacy
        );
        assert_eq!(
            LyricsV2Mode::from_override(None, true),
            LyricsV2Mode::OpenGl
        );
        assert_eq!(
            LyricsV2Mode::from_override(Some("cairo"), true),
            LyricsV2Mode::Cairo
        );
        assert_eq!(
            LyricsV2Mode::from_override(Some("invalid"), true),
            LyricsV2Mode::Legacy,
            "an invalid explicit override must fail closed"
        );
    }

    #[test]
    fn only_shadow_runs_both_parser_and_timeline_paths() {
        assert!(LyricsV2Mode::Legacy.runs_legacy());
        assert!(!LyricsV2Mode::Legacy.runs_v2());
        assert!(LyricsV2Mode::Shadow.runs_legacy());
        assert!(LyricsV2Mode::Shadow.runs_v2());
        assert!(!LyricsV2Mode::Cairo.runs_legacy());
        assert!(!LyricsV2Mode::OpenGl.runs_legacy());
    }
}
