#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Dev,
    Focus,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandMode {
    Collapsed,
    Expanded,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AppStatus {
    pub visibility: Visibility,
    pub mode: IslandMode,
    pub is_animating: bool,
    pub active_task: String,
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            visibility: Visibility::Visible,
            mode: IslandMode::Collapsed,
            is_animating: false,
            active_task: "Idle".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_tab_equality() {
        assert_eq!(ActiveTab::Dev, ActiveTab::Dev);
        assert_ne!(ActiveTab::Dev, ActiveTab::Focus);
        assert_ne!(ActiveTab::Focus, ActiveTab::System);
    }

    #[test]
    fn test_app_status_defaults() {
        let status = AppStatus::default();
        assert_eq!(status.visibility, Visibility::Visible);
        assert_eq!(status.mode, IslandMode::Collapsed);
        assert!(!status.is_animating);
    }
}
