use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTimerStatus {
    Stopped,
    Running,
    Paused,
    Completed,
}

#[derive(Debug, Clone)]
pub struct FocusTimer {
    pub status: FocusTimerStatus,
    pub duration_secs: u32,
    pub remaining_secs: u32,
    pub last_tick: Instant,
}

impl FocusTimer {
    pub fn new(duration_secs: u32) -> Self {
        Self {
            status: FocusTimerStatus::Stopped,
            duration_secs,
            remaining_secs: duration_secs,
            last_tick: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.remaining_secs = self.duration_secs;
        self.status = FocusTimerStatus::Running;
        self.last_tick = Instant::now();
    }

    pub fn pause(&mut self) {
        if self.status == FocusTimerStatus::Running {
            self.status = FocusTimerStatus::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.status == FocusTimerStatus::Paused {
            self.status = FocusTimerStatus::Running;
            self.last_tick = Instant::now();
        }
    }

    pub fn toggle(&mut self) {
        match self.status {
            FocusTimerStatus::Running => self.pause(),
            FocusTimerStatus::Paused => self.resume(),
            FocusTimerStatus::Stopped | FocusTimerStatus::Completed => self.start(),
        }
    }

    pub fn reset(&mut self) {
        self.status = FocusTimerStatus::Stopped;
        self.remaining_secs = self.duration_secs;
    }

    pub fn set_duration(&mut self, secs: u32) {
        self.duration_secs = secs;
        self.remaining_secs = secs;
        self.status = FocusTimerStatus::Stopped;
    }

    pub fn add_seconds(&mut self, secs: u32) {
        self.duration_secs = self.duration_secs.saturating_add(secs);
        self.remaining_secs = self.remaining_secs.saturating_add(secs);
        if self.status == FocusTimerStatus::Completed {
            self.status = FocusTimerStatus::Running;
            self.last_tick = Instant::now();
        }
    }

    pub fn progress_pct(&self) -> f32 {
        if self.duration_secs == 0 {
            return 1.0;
        }
        (1.0 - (self.remaining_secs as f32 / self.duration_secs as f32)).clamp(0.0, 1.0)
    }

    /// Advances the timer. Returns true if remaining seconds changed.
    pub fn tick(&mut self) -> bool {
        if self.status != FocusTimerStatus::Running {
            return false;
        }

        let elapsed = self.last_tick.elapsed().as_secs() as u32;
        if elapsed >= 1 {
            if self.remaining_secs <= elapsed {
                self.remaining_secs = 0;
                self.status = FocusTimerStatus::Completed;
            } else {
                self.remaining_secs -= elapsed;
            }
            self.last_tick = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn format_time(&self) -> String {
        let mins = self.remaining_secs / 60;
        let secs = self.remaining_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    pub fn status_label(&self) -> &'static str {
        match self.status {
            FocusTimerStatus::Stopped => "Ready",
            FocusTimerStatus::Running => "Focus",
            FocusTimerStatus::Paused => "Paused",
            FocusTimerStatus::Completed => "Done! 🎉",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_initialization() {
        let timer = FocusTimer::new(1500);
        assert_eq!(timer.status, FocusTimerStatus::Stopped);
        assert_eq!(timer.remaining_secs, 1500);
        assert_eq!(timer.format_time(), "25:00");
    }

    #[test]
    fn test_timer_toggle_lifecycle() {
        let mut timer = FocusTimer::new(60);
        timer.toggle(); // Start
        assert_eq!(timer.status, FocusTimerStatus::Running);

        timer.toggle(); // Pause
        assert_eq!(timer.status, FocusTimerStatus::Paused);

        timer.toggle(); // Resume
        assert_eq!(timer.status, FocusTimerStatus::Running);

        timer.reset();
        assert_eq!(timer.status, FocusTimerStatus::Stopped);
        assert_eq!(timer.remaining_secs, 60);
    }

    #[test]
    fn test_format_time_boundaries() {
        let mut timer = FocusTimer::new(0);
        assert_eq!(timer.format_time(), "00:00");

        timer.remaining_secs = 65;
        assert_eq!(timer.format_time(), "01:05");

        timer.remaining_secs = 3599;
        assert_eq!(timer.format_time(), "59:59");
    }

    #[test]
    fn test_focus_timer_presets_and_progress() {
        let mut timer = FocusTimer::new(1500); // 25 mins
        assert_eq!(timer.progress_pct(), 0.0);

        timer.set_duration(300); // 5 mins
        assert_eq!(timer.remaining_secs, 300);
        assert_eq!(timer.format_time(), "05:00");

        timer.remaining_secs = 150;
        assert!((timer.progress_pct() - 0.5).abs() < 0.01);

        timer.add_seconds(300);
        assert_eq!(timer.remaining_secs, 450);
        assert_eq!(timer.duration_secs, 600);
    }
}
