use std::ptr;
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

use crate::config::Snippet;

pub struct SnippetManager {
    pub snippets: Vec<Snippet>,
    pub last_copied: Option<String>,
}

impl SnippetManager {
    pub fn new(snippets: Vec<Snippet>) -> Self {
        Self {
            snippets,
            last_copied: None,
        }
    }

    /// Copies a snippet's text to the Windows clipboard.
    /// Does NO background capture or monitoring.
    pub fn copy_to_clipboard(&mut self, text: &str) -> bool {
        unsafe {
            if OpenClipboard(HWND::default()).is_err() {
                eprintln!("Failed to open Windows clipboard");
                return false;
            }

            let _ = EmptyClipboard();

            let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let size_in_bytes = utf16.len() * std::mem::size_of::<u16>();

            let h_global = match GlobalAlloc(GMEM_MOVEABLE, size_in_bytes) {
                Ok(h) => h,
                Err(_) => {
                    let _ = CloseClipboard();
                    return false;
                }
            };

            let dest_ptr = GlobalLock(h_global) as *mut u16;
            if dest_ptr.is_null() {
                let _ = CloseClipboard();
                return false;
            }

            ptr::copy_nonoverlapping(utf16.as_ptr(), dest_ptr, utf16.len());
            let _ = GlobalUnlock(h_global);

            // CF_UNICODETEXT format id is 13
            let res = SetClipboardData(13, HANDLE(h_global.0));
            let _ = CloseClipboard();

            let success = res.is_ok();
            if success {
                self.last_copied = Some(text.to_string());
                println!("Copied snippet to clipboard: {}", text);
            }
            success
        }
    }

    /// Reads the current text from the Windows clipboard on demand.
    pub fn get_clipboard_text() -> Option<String> {
        unsafe {
            use windows::Win32::Foundation::HGLOBAL;
            use windows::Win32::System::DataExchange::GetClipboardData;

            if OpenClipboard(HWND::default()).is_err() {
                return None;
            }

            let handle = match GetClipboardData(13) { // CF_UNICODETEXT
                Ok(h) => h,
                Err(_) => {
                    let _ = CloseClipboard();
                    return None;
                }
            };

            let ptr = GlobalLock(HGLOBAL(handle.0)) as *const u16;
            if ptr.is_null() {
                let _ = CloseClipboard();
                return None;
            }

            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }

            let slice = std::slice::from_raw_parts(ptr, len);
            let text = String::from_utf16_lossy(slice);

            let _ = GlobalUnlock(HGLOBAL(handle.0));
            let _ = CloseClipboard();

            Some(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_manager() {
        let list = vec![
            Snippet { title: "build".to_string(), content: "cargo build".to_string() }
        ];
        let mgr = SnippetManager::new(list);
        assert_eq!(mgr.snippets.len(), 1);
        assert_eq!(mgr.snippets[0].title, "build");
    }
}
