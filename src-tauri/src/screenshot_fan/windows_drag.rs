use image::RgbaImage;
use std::{mem::size_of, path::Path, ptr};
use windows::{
    core::{implement, Error, Owned, Result, BOOL, HRESULT, HSTRING, PCWSTR},
    Win32::{
        Foundation::{DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, S_OK},
        Graphics::Gdi::{
            CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, SelectObject, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        },
        System::{
            Com::IDataObject,
            Ole::{
                DoDragDrop, IDropSource, IDropSource_Impl, OleInitialize, OleUninitialize,
                DROPEFFECT, DROPEFFECT_COPY,
            },
            SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS},
        },
        UI::{
            Shell::{
                BHID_DataObject, IShellItem, IShellItemArray, SHCreateItemFromParsingName,
                SHCreateShellItemArrayFromShellItem,
            },
            WindowsAndMessaging::{
                CreateIconIndirect, DrawIconEx, GetCursor, GetSystemMetrics, LoadCursorW,
                SetCursor, DI_NORMAL, HCURSOR, HICON, ICONINFO, IDC_ARROW, IDC_NO, SM_CXCURSOR,
                SM_CYCURSOR,
            },
        },
    },
};

// The card and the pointer are one OS cursor, even over targets that suppress shell drag images.
#[implement(IDropSource)]
struct CardSource {
    allowed: Owned<HCURSOR>,
    blocked: Owned<HCURSOR>,
}

#[allow(non_snake_case)]
impl IDropSource_Impl for CardSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            DRAGDROP_S_CANCEL
        } else if !keys.contains(MK_LBUTTON) {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, effect: DROPEFFECT) -> HRESULT {
        let cursor = if effect.contains(DROPEFFECT_COPY) {
            &self.allowed
        } else {
            &self.blocked
        };
        unsafe {
            SetCursor(Some(**cursor));
        }
        S_OK
    }
}

fn card_cursor(pixels: &RgbaImage, x: i32, y: i32, pointer: PCWSTR) -> Result<Owned<HCURSOR>> {
    unsafe {
        let glyph = LoadCursorW(None, pointer)?;
        let glyph_w = GetSystemMetrics(SM_CXCURSOR);
        let glyph_h = GetSystemMetrics(SM_CYCURSOR);
        let width = (pixels.width() as i32).max(x + glyph_w);
        let height = (pixels.height() as i32).max(y + glyph_h);
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = ptr::null_mut();
        let color = Owned::new(CreateDIBSection(
            None,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )?);
        let dst =
            std::slice::from_raw_parts_mut(bits.cast::<u8>(), width as usize * height as usize * 4);
        dst.fill(0);
        for (row, target) in pixels.rows().zip(dst.chunks_exact_mut(width as usize * 4)) {
            for (pixel, bgra) in row.zip(target.chunks_exact_mut(4)) {
                let [r, g, b, a] = pixel.0;
                let premul = |v: u8| ((v as u16 * a as u16 + 127) / 255) as u8;
                bgra.copy_from_slice(&[premul(b), premul(g), premul(r), a]);
            }
        }
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Err(Error::from_thread());
        }
        let old = SelectObject(dc, (*color).into());
        let drawn = DrawIconEx(
            dc,
            x,
            y,
            HICON(glyph.0),
            glyph_w,
            glyph_h,
            0,
            None,
            DI_NORMAL,
        );
        SelectObject(dc, old);
        let _ = DeleteDC(dc);
        drawn?;
        let mask = Owned::new(CreateBitmap(width, height, 1, 1, None));
        if mask.is_invalid() {
            return Err(Error::from_thread());
        }
        let icon = CreateIconIndirect(&ICONINFO {
            xHotspot: x as u32,
            yHotspot: y as u32,
            hbmMask: *mask,
            hbmColor: *color,
            ..Default::default()
        })?;
        Ok(Owned::new(HCURSOR(icon.0)))
    }
}

pub(super) fn drag_file(path: &Path, pixels: RgbaImage, x: i32, y: i32) -> Result<()> {
    let source: IDropSource = CardSource {
        allowed: card_cursor(&pixels, x, y, IDC_ARROW)?,
        blocked: card_cursor(&pixels, x, y, IDC_NO)?,
    }
    .into();
    unsafe {
        OleInitialize(None)?;
        let result = (|| {
            let item: IShellItem =
                SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None)?;
            let items: IShellItemArray = SHCreateShellItemArrayFromShellItem(&item)?;
            let data: IDataObject = items.BindToHandler(None, &BHID_DataObject)?;
            let previous = GetCursor();
            let result = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut DROPEFFECT::default());
            SetCursor(Some(previous));
            result.ok()
        })();
        OleUninitialize();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::{
        Graphics::Gdi::{GetObjectW, BITMAP},
        UI::WindowsAndMessaging::GetIconInfo,
    };

    #[test]
    fn system_cursor_preserves_card_size_and_off_center_grip() -> Result<()> {
        let card = RgbaImage::from_pixel(214, 170, image::Rgba([210, 200, 190, 255]));
        let (x, y) = (183, 41);
        for pointer in [IDC_ARROW, IDC_NO] {
            let cursor = card_cursor(&card, x, y, pointer)?;
            unsafe {
                let mut info = ICONINFO::default();
                GetIconInfo(HICON(cursor.0), &mut info)?;
                let color = Owned::new(info.hbmColor);
                let _mask = Owned::new(info.hbmMask);
                assert!(!info.fIcon.as_bool());
                assert_eq!((info.xHotspot, info.yHotspot), (x as u32, y as u32));
                let mut bitmap = BITMAP::default();
                assert_ne!(
                    GetObjectW(
                        (*color).into(),
                        size_of::<BITMAP>() as i32,
                        Some(ptr::from_mut(&mut bitmap).cast())
                    ),
                    0
                );
                assert!(
                    bitmap.bmWidth >= card.width() as i32
                        && bitmap.bmHeight >= card.height() as i32
                );
            }
        }
        Ok(())
    }
}
