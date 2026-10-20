use std::ptr;
use windows::Win32::Foundation::{COLORREF, HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HBITMAP,
    BLENDFUNCTION, AC_SRC_OVER, AC_SRC_ALPHA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    UpdateLayeredWindow, ULW_ALPHA,
};

pub struct DibBuffer {
    dc: HDC,
    bitmap: HBITMAP,
    bits: *mut u8,
    width: i32,
    height: i32,
}

impl DibBuffer {
    pub fn new(width: i32, height: i32) -> Result<Self, windows::core::Error> {
        unsafe {
            let dc = CreateCompatibleDC(HDC::default());

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // Top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }],
            };

            let mut bits: *mut std::ffi::c_void = ptr::null_mut();
            let bitmap = CreateDIBSection(
                dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            )?;

            SelectObject(dc, bitmap);

            Ok(Self {
                dc,
                bitmap,
                bits: bits as *mut u8,
                width,
                height,
            })
        }
    }

    pub fn update_layered_window(
        &mut self,
        hwnd: HWND,
        rgba_data: &[u8],
        pos_x: i32,
        pos_y: i32,
    ) {
        let total_pixels = (self.width * self.height) as usize;
        let expected_bytes = total_pixels * 4;

        if rgba_data.len() < expected_bytes || self.bits.is_null() {
            return;
        }

        unsafe {
            let dst = std::slice::from_raw_parts_mut(self.bits, expected_bytes);
            // Convert RGBA -> BGRA for Win32 Layered Window
            for i in 0..total_pixels {
                let offset = i * 4;
                let r = rgba_data[offset];
                let g = rgba_data[offset + 1];
                let b = rgba_data[offset + 2];
                let a = rgba_data[offset + 3];

                dst[offset] = b;
                dst[offset + 1] = g;
                dst[offset + 2] = r;
                dst[offset + 3] = a;
            }

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let pt_dst = POINT { x: pos_x, y: pos_y };
            let size = SIZE { cx: self.width, cy: self.height };
            let pt_src = POINT { x: 0, y: 0 };

            let _ = UpdateLayeredWindow(
                hwnd,
                HDC::default(),
                Some(&pt_dst),
                Some(&size),
                self.dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for DibBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(self.bitmap);
            }
            if !self.dc.is_invalid() {
                let _ = DeleteDC(self.dc);
            }
        }
    }
}
