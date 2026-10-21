//! Developer Utilities & Scratchpad
//!
//! Provides essential developer micro-tools directly in the Dynamic Island:
//! - JSON Formatter & Validator (pretty-print, validate, minify)
//! - JWT Formatter & Inspector (decode header & payload, check expiration)
//! - RFC 4122 UUID v4 Generator (native Windows CoCreateGuid)
//! - Screen Hex / RGB Color Picker (native Win32 GetPixel under cursor)

use windows::Win32::Foundation::{COLORREF, HWND, POINT};
use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoCreateGuid(pguid: *mut windows::core::GUID) -> i32;
}

/// Result of inspecting a JSON Web Token (JWT).
#[derive(Debug, Clone, PartialEq)]
pub struct JwtDetails {
    pub header_json: String,
    pub payload_json: String,
    pub signature_preview: String,
    pub subject: Option<String>,
    pub issuer: Option<String>,
    pub expires_at_unix: Option<i64>,
    pub expiration_status: Option<String>,
}

/// Color sampled from the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenColor {
    pub hex: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub rgb_str: String,
    pub cursor_x: i32,
    pub cursor_y: i32,
}

pub struct DeveloperUtilities;

#[allow(dead_code)]
impl DeveloperUtilities {
    // =========================================================================
    // 1. JSON Utilities
    // =========================================================================

    /// Pretty-prints a JSON string with 2-space indentation.
    /// Returns an error string with line/col details if JSON is invalid.
    pub fn format_json(input: &str) -> Result<String, String> {
        let val: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        serde_json::to_string_pretty(&val)
            .map_err(|e| format!("Format error: {}", e))
    }

    /// Validates whether the given string is valid JSON.
    pub fn validate_json(input: &str) -> Result<(), String> {
        serde_json::from_str::<serde_json::Value>(input)
            .map(|_| ())
            .map_err(|e| format!("Invalid JSON: {}", e))
    }

    /// Minifies a JSON string to remove all unnecessary whitespace.
    pub fn minify_json(input: &str) -> Result<String, String> {
        let val: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        serde_json::to_string(&val)
            .map_err(|e| format!("Minify error: {}", e))
    }

    // =========================================================================
    // 2. JWT Inspector
    // =========================================================================

    /// Decodes a JWT token string into formatted header and payload JSON with claims.
    pub fn inspect_jwt(token: &str) -> Result<JwtDetails, String> {
        let parts: Vec<&str> = token.trim().split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT: token must have 3 parts separated by dots".to_string());
        }

        let header_bytes = Self::base64_url_decode(parts[0])
            .map_err(|e| format!("Failed to decode JWT header: {}", e))?;
        let header_str = std::str::from_utf8(&header_bytes)
            .map_err(|e| format!("JWT header is not valid UTF-8: {}", e))?;
        let header_val: serde_json::Value = serde_json::from_str(header_str)
            .map_err(|e| format!("JWT header is not valid JSON: {}", e))?;
        let header_json = serde_json::to_string_pretty(&header_val)
            .unwrap_or_else(|_| header_str.to_string());

        let payload_bytes = Self::base64_url_decode(parts[1])
            .map_err(|e| format!("Failed to decode JWT payload: {}", e))?;
        let payload_str = std::str::from_utf8(&payload_bytes)
            .map_err(|e| format!("JWT payload is not valid UTF-8: {}", e))?;
        let payload_val: serde_json::Value = serde_json::from_str(payload_str)
            .map_err(|e| format!("JWT payload is not valid JSON: {}", e))?;
        let payload_json = serde_json::to_string_pretty(&payload_val)
            .unwrap_or_else(|_| payload_str.to_string());

        let subject = payload_val.get("sub").and_then(|v| v.as_str()).map(String::from);
        let issuer = payload_val.get("iss").and_then(|v| v.as_str()).map(String::from);
        let exp = payload_val.get("exp").and_then(|v| v.as_i64());

        let expiration_status = exp.map(|exp_secs| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            if exp_secs < now {
                let diff = now - exp_secs;
                if diff < 60 {
                    format!("Expired {}s ago", diff)
                } else if diff < 3600 {
                    format!("Expired {}m ago", diff / 60)
                } else if diff < 86400 {
                    format!("Expired {}h ago", diff / 3600)
                } else {
                    format!("Expired {}d ago", diff / 86400)
                }
            } else {
                let diff = exp_secs - now;
                if diff < 60 {
                    format!("Expires in {}s", diff)
                } else if diff < 3600 {
                    format!("Expires in {}m", diff / 60)
                } else if diff < 86400 {
                    format!("Expires in {}h", diff / 3600)
                } else {
                    format!("Expires in {}d", diff / 86400)
                }
            }
        });

        let sig = parts[2];
        let sig_preview = if sig.len() > 16 {
            format!("{}...{}", &sig[..8], &sig[sig.len() - 8..])
        } else {
            sig.to_string()
        };

        Ok(JwtDetails {
            header_json,
            payload_json,
            signature_preview: sig_preview,
            subject,
            issuer,
            expires_at_unix: exp,
            expiration_status,
        })
    }

    /// Pure Rust Base64URL decoder (RFC 4648 without external dependencies).
    pub fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
        let mut base64_str = input.replace('-', "+").replace('_', "/");
        let remainder = base64_str.len() % 4;
        if remainder > 0 {
            base64_str.push_str(&"=".repeat(4 - remainder));
        }

        Self::base64_standard_decode(&base64_str)
    }

    fn base64_standard_decode(input: &str) -> Result<Vec<u8>, String> {
        fn val(c: u8) -> Result<u8, String> {
            match c {
                b'A'..=b'Z' => Ok(c - b'A'),
                b'a'..=b'z' => Ok(c - b'a' + 26),
                b'0'..=b'9' => Ok(c - b'0' + 52),
                b'+' => Ok(62),
                b'/' => Ok(63),
                b'=' => Ok(0),
                _ => Err(format!("Invalid base64 character: {}", c as char)),
            }
        }

        let bytes = input.as_bytes();
        if bytes.len() % 4 != 0 {
            return Err("Invalid base64 length".to_string());
        }

        let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
        let mut i = 0;
        while i < bytes.len() {
            let b0 = val(bytes[i])?;
            let b1 = val(bytes[i + 1])?;
            let b2 = val(bytes[i + 2])?;
            let b3 = val(bytes[i + 3])?;

            out.push((b0 << 2) | (b1 >> 4));
            if bytes[i + 2] != b'=' {
                out.push(((b1 & 0xF) << 4) | (b2 >> 2));
            }
            if bytes[i + 3] != b'=' {
                out.push(((b2 & 0x3) << 6) | b3);
            }

            i += 4;
        }

        Ok(out)
    }

    // =========================================================================
    // 3. UUID Generator
    // =========================================================================

    /// Generates a standard RFC 4122 UUID v4 using the native Windows COM `CoCreateGuid`.
    /// Format: `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx` (lowercase).
    pub fn generate_uuid_v4() -> String {
        let mut guid = windows::core::GUID::zeroed();
        unsafe {
            let _ = CoCreateGuid(&mut guid);
        }

        // Format as canonical 8-4-4-4-12 lowercase hex string
        format!(
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            guid.data1,
            guid.data2,
            guid.data3,
            guid.data4[0],
            guid.data4[1],
            guid.data4[2],
            guid.data4[3],
            guid.data4[4],
            guid.data4[5],
            guid.data4[6],
            guid.data4[7],
        )
    }

    // =========================================================================
    // 4. Screen Color Picker
    // =========================================================================

    /// Samples the pixel color directly beneath the mouse cursor across any monitor.
    pub fn pick_screen_color_at_cursor() -> Result<ScreenColor, String> {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() {
                return Err("Failed to retrieve cursor position".to_string());
            }

            // Get DC for the entire virtual desktop
            let hdc = GetDC(HWND(std::ptr::null_mut()));
            if hdc.is_invalid() {
                return Err("Failed to get desktop device context".to_string());
            }

            let pixel: COLORREF = GetPixel(hdc, pt.x, pt.y);
            let _ = ReleaseDC(HWND(std::ptr::null_mut()), hdc);

            if pixel.0 == 0xFFFFFFFF {
                return Err("Failed to sample pixel color at coordinates".to_string());
            }

            // COLORREF format is 0x00BBGGRR
            let r = (pixel.0 & 0x000000FF) as u8;
            let g = ((pixel.0 & 0x0000FF00) >> 8) as u8;
            let b = ((pixel.0 & 0x00FF0000) >> 16) as u8;

            let hex = format!("#{:02X}{:02X}{:02X}", r, g, b);
            let rgb_str = format!("rgb({}, {}, {})", r, g, b);

            Ok(ScreenColor {
                hex,
                r,
                g,
                b,
                rgb_str,
                cursor_x: pt.x,
                cursor_y: pt.y,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_format_and_validate() {
        let raw = r#"{"name":"island","dev":true,"ports":[3000,5173]}"#;
        let formatted = DeveloperUtilities::format_json(raw).expect("Should format");
        assert!(formatted.contains("  \"name\": \"island\""));
        assert!(formatted.contains("  \"ports\": [\n    3000,\n    5173\n  ]"));

        let invalid = r#"{"name": "broken",}"#;
        assert!(DeveloperUtilities::validate_json(invalid).is_err());
        assert!(DeveloperUtilities::validate_json(raw).is_ok());

        let minified = DeveloperUtilities::minify_json(&formatted).expect("Should minify");
        let parsed_minified: serde_json::Value = serde_json::from_str(&minified).unwrap();
        let parsed_raw: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed_minified, parsed_raw);
    }

    #[test]
    fn test_uuid_v4_generation() {
        let u1 = DeveloperUtilities::generate_uuid_v4();
        let u2 = DeveloperUtilities::generate_uuid_v4();
        assert_eq!(u1.len(), 36);
        assert_eq!(u2.len(), 36);
        assert_ne!(u1, u2);
        // Verify UUID v4 format xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        let parts: Vec<&str> = u1.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        // Character at index 0 of part 2 should be '4' (UUID v4)
        assert_eq!(&parts[2][0..1], "4");
    }

    #[test]
    fn test_jwt_inspector() {
        // Sample standard test JWT with {"alg":"HS256","typ":"JWT"} and {"sub":"1234567890","name":"John Doe","exp":9999999999}
        // base64url header: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9
        // base64url payload: eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiZXhwIjo5OTk5OTk5OTk5fQ
        let sample_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiZXhwIjo5OTk5OTk5OTk5fQ.sample_signature_abc123";
        let details = DeveloperUtilities::inspect_jwt(sample_jwt).expect("Should parse JWT");
        assert_eq!(details.subject.as_deref(), Some("1234567890"));
        assert!(details.header_json.contains("HS256"));
        assert!(details.payload_json.contains("John Doe"));
        assert_eq!(details.expires_at_unix, Some(9999999999));
        assert!(details.expiration_status.as_ref().unwrap().starts_with("Expires in"));
    }

    #[test]
    fn test_color_picker_logic() {
        // Pixel color conversion test
        let colorref_val: u32 = 0x00E2904A; // 0x00BBGGRR -> R=0x4A, G=0x90, B=0xE2
        let r = (colorref_val & 0xFF) as u8;
        let g = ((colorref_val & 0xFF00) >> 8) as u8;
        let b = ((colorref_val & 0xFF0000) >> 16) as u8;
        assert_eq!(format!("#{:02X}{:02X}{:02X}", r, g, b), "#4A90E2");
        assert_eq!(format!("rgb({}, {}, {})", r, g, b), "rgb(74, 144, 226)");
    }
}
