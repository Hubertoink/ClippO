use image::RgbaImage;
use std::{io, mem::size_of, ptr};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, SIZE},
    Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
        AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetCursorPos, GetWindowLongPtrW, KillTimer, SetTimer,
        SetWindowLongPtrW, SetWindowPos, UpdateLayeredWindow, GWLP_USERDATA, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, ULW_ALPHA, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
    },
};

// Some targets (including Word) suppress the shell's drag image. Keep the card
// on a nonactivating, click-through surface while OLE handles the actual file.
pub(super) struct LiftedCard(HWND);

impl LiftedCard {
    pub fn show(mut pixels: RgbaImage, x: i32, y: i32) -> io::Result<Self> {
        let size = SIZE {
            cx: pixels.width() as i32,
            cy: pixels.height() as i32,
        };
        for p in pixels.pixels_mut() {
            let [r, g, b, a] = p.0;
            let premultiply = |v: u8| ((v as u16 * a as u16 + 127) / 255) as u8;
            p.0 = [premultiply(b), premultiply(g), premultiply(r), a];
        }
        unsafe {
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                windows_sys::w!("STATIC"),
                ptr::null(),
                WS_POPUP,
                0,
                0,
                size.cx,
                size.cy,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            );
            if hwnd.is_null() {
                return Err(io::Error::last_os_error());
            }
            let card = Self(hwnd);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size.cx,
                    biHeight: -size.cy,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..Default::default()
                },
                ..Default::default()
            };
            let dc = CreateCompatibleDC(ptr::null_mut());
            if dc.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut bits = ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
            if bitmap.is_null() {
                let error = io::Error::last_os_error();
                DeleteDC(dc);
                return Err(error);
            }
            ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast(), pixels.as_raw().len());
            let old = SelectObject(dc, bitmap);
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let updated = UpdateLayeredWindow(
                hwnd,
                ptr::null_mut(),
                ptr::null(),
                &size,
                dc,
                &POINT::default(),
                0,
                &blend,
                ULW_ALPHA,
            );
            let error = (updated == 0).then(io::Error::last_os_error);
            SelectObject(dc, old);
            DeleteObject(bitmap);
            DeleteDC(dc);
            if let Some(error) = error {
                return Err(error);
            }

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (x | (y << 16)) as isize);
            let mut cursor = POINT::default();
            if GetCursorPos(&mut cursor) == 0
                || SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    cursor.x - x,
                    cursor.y - y,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOSIZE | SWP_SHOWWINDOW,
                ) == 0
                || SetTimer(hwnd, 1, 10, Some(follow_pointer)) == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(card)
        }
    }
}

unsafe extern "system" fn follow_pointer(hwnd: HWND, _: u32, _: usize, _: u32) {
    let grip = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as i32;
    let mut cursor = POINT::default();
    if GetCursorPos(&mut cursor) != 0 {
        SetWindowPos(
            hwnd,
            ptr::null_mut(),
            cursor.x - (grip & 0xffff),
            cursor.y - (grip >> 16),
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        );
    }
}

impl Drop for LiftedCard {
    fn drop(&mut self) {
        unsafe {
            KillTimer(self.0, 1);
            DestroyWindow(self.0);
        }
    }
}
