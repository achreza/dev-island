use std::env;
use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, REG_VALUE_TYPE,
};

const RUN_SUBKEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_REG_NAME: PCWSTR = w!("DynamicIslandDev");

pub struct AutostartManager;

impl AutostartManager {
    pub fn is_enabled() -> bool {
        unsafe {
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                RUN_SUBKEY,
                0,
                KEY_READ,
                &mut hkey,
            ).is_err() {
                return false;
            }

            let mut value_type = REG_VALUE_TYPE::default();
            let mut data_len = 0u32;
            let res = RegQueryValueExW(
                hkey,
                APP_REG_NAME,
                None,
                Some(&mut value_type),
                None,
                Some(&mut data_len),
            );

            let _ = RegCloseKey(hkey);
            res.is_ok() && value_type == REG_SZ
        }
    }

    pub fn set_enabled(enabled: bool) -> bool {
        unsafe {
            let mut hkey = HKEY::default();
            if RegOpenKeyExW(
                HKEY_CURRENT_USER,
                RUN_SUBKEY,
                0,
                KEY_WRITE,
                &mut hkey,
            ).is_err() {
                return false;
            }

            let success = if enabled {
                if let Ok(exe_path) = env::current_exe() {
                    let path_str = exe_path.to_string_lossy().to_string();
                    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
                    let byte_len = (wide_path.len() * std::mem::size_of::<u16>()) as u32;

                    RegSetValueExW(
                        hkey,
                        APP_REG_NAME,
                        0,
                        REG_SZ,
                        Some(std::slice::from_raw_parts(wide_path.as_ptr() as *const u8, byte_len as usize)),
                    ).is_ok()
                } else {
                    false
                }
            } else {
                let _ = RegDeleteValueW(hkey, APP_REG_NAME);
                true
            };

            let _ = RegCloseKey(hkey);
            success
        }
    }

    pub fn toggle() -> bool {
        let currently_enabled = Self::is_enabled();
        Self::set_enabled(!currently_enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostart_query_does_not_panic() {
        // Querying whether autostart is enabled should succeed and never panic
        let _ = AutostartManager::is_enabled();
    }
}
