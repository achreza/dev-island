#[derive(Debug, Clone)]
pub struct TaskNote {
    pub text: String,
}

impl TaskNote {
    pub fn new(initial: &str) -> Self {
        Self {
            text: initial.to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn set(&mut self, text: &str) {
        self.text = text.trim().to_string();
    }

    pub fn display_summary(&self, max_chars: usize) -> String {
        if self.text.chars().count() <= max_chars {
            self.text.clone()
        } else {
            let truncated: String = self.text.chars().take(max_chars.saturating_sub(3)).collect();
            format!("{}...", truncated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_note_truncation() {
        let note = TaskNote::new("Short note");
        assert_eq!(note.display_summary(20), "Short note");

        let long_note = TaskNote::new("This is a very long developer task description");
        assert_eq!(long_note.display_summary(15), "This is a ve...");
    }
}
