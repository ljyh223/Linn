/// The lyric surface is an explicit product choice, not something inferred
/// from foreground or album colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricsPresentation {
    /// Compact reading surface on a solid application background.
    Sidebar,
    /// Immersive surface above album-derived animated colour.
    Fullscreen,
}

impl LyricsPresentation {
    pub fn uses_album_palette(self) -> bool {
        self == Self::Fullscreen
    }

    /// Opacity of a line that has already been sung.
    pub fn past_line_opacity(self) -> f32 {
        match self {
            Self::Sidebar => 0.42,
            Self::Fullscreen => 0.48,
        }
    }

    /// Opacity of a line whose timing has not started yet.
    pub fn future_line_opacity(self) -> f32 {
        match self {
            Self::Sidebar => 0.24,
            Self::Fullscreen => 0.20,
        }
    }

    /// Base opacity below the bright karaoke reveal on the active line.
    pub fn karaoke_unsung_opacity(self) -> f32 {
        match self {
            Self::Sidebar => 0.26,
            Self::Fullscreen => 0.20,
        }
    }

    pub fn inactive_scale(self) -> f64 {
        match self {
            Self::Sidebar => 0.99,
            Self::Fullscreen => 0.98,
        }
    }

    pub fn line_scale(self, focus_mix: f64) -> f64 {
        lerp(self.inactive_scale(), 1.0, focus_mix.clamp(0.0, 1.0))
    }

    pub fn line_base_opacity(
        self,
        index: usize,
        focus: Option<usize>,
        is_active_karaoke: bool,
        focus_mix: f64,
    ) -> f32 {
        let mix = focus_mix.clamp(0.0, 1.0) as f32;
        if is_active_karaoke {
            return lerp_f32(
                self.karaoke_unsung_opacity() * self.past_line_opacity(),
                self.karaoke_unsung_opacity(),
                mix,
            );
        }

        match focus {
            Some(focus) if index < focus => lerp_f32(self.past_line_opacity(), 1.0, mix),
            _ => lerp_f32(self.future_line_opacity(), self.past_line_opacity(), mix),
        }
    }

    pub fn active_overlay_opacity(self, focus_mix: f64) -> f32 {
        lerp_f32(
            self.past_line_opacity(),
            1.0,
            focus_mix.clamp(0.0, 1.0) as f32,
        )
    }

    pub fn translation_opacity(self, index: usize, focus: Option<usize>, focus_mix: f64) -> f32 {
        let resting = match focus {
            Some(focus) if index < focus => 0.34,
            _ => 0.20,
        };
        lerp_f32(resting, 0.58, focus_mix.clamp(0.0, 1.0) as f32)
    }

    pub fn horizontal_padding(self) -> f32 {
        match self {
            Self::Sidebar => 24.0,
            Self::Fullscreen => 36.0,
        }
    }
}

fn lerp(from: f64, to: f64, amount: f64) -> f64 {
    from + (to - from) * amount
}

fn lerp_f32(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fullscreen_uses_album_palette() {
        assert!(!LyricsPresentation::Sidebar.uses_album_palette());
        assert!(LyricsPresentation::Fullscreen.uses_album_palette());
    }

    #[test]
    fn karaoke_has_a_clear_sung_unsung_hierarchy() {
        assert!(
            LyricsPresentation::Sidebar.karaoke_unsung_opacity()
                < LyricsPresentation::Sidebar.past_line_opacity()
        );
        assert!(
            LyricsPresentation::Fullscreen.karaoke_unsung_opacity()
                < LyricsPresentation::Fullscreen.past_line_opacity()
        );
    }

    #[test]
    fn future_lines_are_quieter_than_past_lines() {
        for presentation in [LyricsPresentation::Sidebar, LyricsPresentation::Fullscreen] {
            assert!(presentation.future_line_opacity() < presentation.past_line_opacity());
        }
    }

    #[test]
    fn focus_mix_animates_scale_and_line_emphasis() {
        let presentation = LyricsPresentation::Fullscreen;
        assert_eq!(presentation.line_scale(0.0), presentation.inactive_scale());
        assert_eq!(presentation.line_scale(1.0), 1.0);
        assert_eq!(presentation.line_base_opacity(1, Some(2), false, 1.0), 1.0);
        assert_eq!(presentation.active_overlay_opacity(1.0), 1.0);
        assert!(
            presentation.translation_opacity(3, Some(2), 0.0)
                < presentation.translation_opacity(1, Some(2), 0.0)
        );
    }
}
