use std::time::Instant;

/// Easing functions for smooth transitions.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[allow(dead_code)]
pub fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t.powi(3)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandState {
    Collapsed,
    Expanded,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub current_progress: f32, // 0.0 (collapsed) to 1.0 (expanded)
    pub start_progress: f32,
    pub target_progress: f32,
    pub start_time: Instant,
    pub duration_secs: f32,
    pub is_animating: bool,
}

impl Animation {
    pub fn new() -> Self {
        Self {
            current_progress: 0.0,
            start_progress: 0.0,
            target_progress: 0.0,
            start_time: Instant::now(),
            duration_secs: 0.28, // 280ms duration
            is_animating: false,
        }
    }

    pub fn expand(&mut self) {
        if self.target_progress != 1.0 {
            self.start_progress = self.current_progress;
            self.target_progress = 1.0;
            self.start_time = Instant::now();
            self.is_animating = true;
        }
    }

    pub fn collapse(&mut self) {
        if self.target_progress != 0.0 {
            self.start_progress = self.current_progress;
            self.target_progress = 0.0;
            self.start_time = Instant::now();
            self.is_animating = true;
        }
    }

    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        if self.target_progress >= 0.5 {
            self.collapse();
        } else {
            self.expand();
        }
    }

    /// Advances the animation according to elapsed real time.
    /// Returns true if an update occurred and the animation is still running.
    pub fn update(&mut self) -> bool {
        if !self.is_animating {
            return false;
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();
        let raw_t = (elapsed / self.duration_secs).clamp(0.0, 1.0);

        // Apply easing depending on direction
        let eased_t = if self.target_progress > self.start_progress {
            ease_out_cubic(raw_t)
        } else {
            ease_in_out_cubic(raw_t)
        };

        self.current_progress = self.start_progress + (self.target_progress - self.start_progress) * eased_t;

        if raw_t >= 1.0 {
            self.current_progress = self.target_progress;
            self.is_animating = false;
        }

        true
    }

    /// Computes current animated pill dimensions given scale.
    /// Collapsed: (200 * scale, 36 * scale)
    /// Expanded:  (400 * scale, 175 * scale)
    pub fn current_dimensions(&self, scale: f32) -> (f32, f32) {
        let collapsed_w = 200.0 * scale;
        let collapsed_h = 36.0 * scale;
        let expanded_w = 400.0 * scale;
        let expanded_h = 175.0 * scale;

        let w = collapsed_w + (expanded_w - collapsed_w) * self.current_progress;
        let h = collapsed_h + (expanded_h - collapsed_h) * self.current_progress;
        (w, h)
    }

    /// Computes current corner radius.
    /// Collapsed is a full capsule (height / 2), expanded is a smooth rounded card.
    pub fn current_corner_radius(&self, scale: f32) -> f32 {
        let collapsed_r = 18.0 * scale;
        let expanded_r = 22.0 * scale;
        collapsed_r + (expanded_r - collapsed_r) * self.current_progress
    }

    #[allow(dead_code)]
    pub fn state(&self) -> IslandState {
        if self.target_progress >= 0.5 {
            IslandState::Expanded
        } else {
            IslandState::Collapsed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_boundaries() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert_eq!(ease_in_cubic(0.0), 0.0);
        assert_eq!(ease_in_cubic(1.0), 1.0);
        assert_eq!(ease_in_out_cubic(0.0), 0.0);
        assert_eq!(ease_in_out_cubic(1.0), 1.0);
    }

    #[test]
    fn test_expand_and_collapse_toggle() {
        let mut anim = Animation::new();
        assert_eq!(anim.current_progress, 0.0);
        assert!(!anim.is_animating);

        anim.expand();
        assert!(anim.is_animating);
        assert_eq!(anim.target_progress, 1.0);

        anim.collapse();
        assert!(anim.is_animating);
        assert_eq!(anim.target_progress, 0.0);
    }

    #[test]
    fn test_dimensions_interpolation() {
        let mut anim = Animation::new();
        let (w0, h0) = anim.current_dimensions(1.0);
        assert_eq!(w0, 200.0);
        assert_eq!(h0, 36.0);

        anim.current_progress = 1.0;
        let (w1, h1) = anim.current_dimensions(1.0);
        assert_eq!(w1, 400.0);
        assert_eq!(h1, 175.0);
    }
}
