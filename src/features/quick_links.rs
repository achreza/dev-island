use std::path::Path;
use windows::core::HSTRING;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::config::QuickLink;

pub struct QuickLinkLauncher;

impl QuickLinkLauncher {
    pub fn is_valid_url(target: &str) -> bool {
        target.starts_with("http://") || target.starts_with("https://")
    }

    pub fn is_safe_local_target(target: &str) -> bool {
        // Prevent shell execution exploits
        if target.contains("&&") || target.contains("||") || target.contains(';') || target.contains('|') {
            return false;
        }

        if target == "." {
            return true;
        }

        let path = Path::new(target);
        if path.exists() {
            return true;
        }

        // Standard Windows executables
        matches!(target.to_lowercase().as_str(), "wt.exe" | "cmd.exe" | "code.exe" | "notepad.exe" | "explorer.exe")
    }

    pub fn launch(link: &QuickLink) -> bool {
        if link.is_url {
            if !Self::is_valid_url(&link.target) {
                eprintln!("Rejected invalid URL: {}", link.target);
                return false;
            }
        } else if !Self::is_safe_local_target(&link.target) {
            eprintln!("Rejected unsafe or non-existent target: {}", link.target);
            return false;
        }

        unsafe {
            let op = HSTRING::from("open");
            let target = HSTRING::from(&link.target);
            let res = ShellExecuteW(
                None,
                &op,
                &target,
                None,
                None,
                SW_SHOWNORMAL,
            );

            let ok = (res.0 as usize) > 32;
            if ok {
                println!("Launched link: {} ({})", link.title, link.target);
            } else {
                eprintln!("Failed to launch link: {} ({})", link.title, link.target);
            }
            ok
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        assert!(QuickLinkLauncher::is_valid_url("https://github.com"));
        assert!(QuickLinkLauncher::is_valid_url("http://localhost:3000"));
        assert!(!QuickLinkLauncher::is_valid_url("file:///etc/passwd"));
        assert!(!QuickLinkLauncher::is_valid_url("javascript:alert(1)"));
    }

    #[test]
    fn test_safe_local_validation() {
        assert!(QuickLinkLauncher::is_safe_local_target("wt.exe"));
        assert!(QuickLinkLauncher::is_safe_local_target("."));
        assert!(!QuickLinkLauncher::is_safe_local_target("calc.exe && notepad.exe"));
        assert!(!QuickLinkLauncher::is_safe_local_target("dir | findstr /i test"));
    }
}
