use crate::engine::{AppResult, Explain};
use arboard::Clipboard;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tempfile::TempDir;

const CAPACITY: usize = 6;

#[derive(Clone, Serialize)]
pub struct Preview {
    id: u64,
    name: String,
    thumbnail: String,
}

struct Entry {
    preview: Preview,
    digest: u64,
    file: PathBuf,
    _directory: TempDir,
}

#[derive(Default)]
pub struct History {
    enabled: bool,
    generation: u64,
    sequence: u64,
    entries: Vec<Arc<Entry>>,
}

impl History {
    fn previews(&self) -> Vec<Preview> {
        self.entries.iter().map(|e| e.preview.clone()).collect()
    }
}

#[tauri::command]
pub fn configure_screenshot_fan(app: AppHandle, enabled: bool) -> AppResult<Vec<Preview>> {
    let state = app.state::<Mutex<History>>();
    let mut history = state.lock().explain()?;
    if history.enabled != enabled {
        history.enabled = enabled;
        history.generation += 1;
        if !enabled {
            history.entries.clear();
        }
    }
    Ok(history.previews())
}

pub fn remember(app: &AppHandle, path: Option<&Path>) -> AppResult<()> {
    let state = app.state::<Mutex<History>>();
    let (id, generation) = {
        let mut history = state.lock().explain()?;
        if !history.enabled {
            return Ok(());
        }
        history.sequence += 1;
        (history.sequence, history.generation)
    };
    let pixels = if let Some(path) = path {
        image::open(path).explain()?.into_rgba8()
    } else {
        let mut clipboard = Clipboard::new().explain()?;
        let data = clipboard.get_image().explain()?;
        RgbaImage::from_raw(
            data.width as u32,
            data.height as u32,
            data.bytes.into_owned(),
        )
        .ok_or("Invalid clipboard image")?
    };
    let mut hash = DefaultHasher::new();
    pixels.dimensions().hash(&mut hash);
    pixels.as_raw().hash(&mut hash);
    let digest = hash.finish();
    {
        let history = state.lock().explain()?;
        if history.generation != generation
            || history.entries.first().is_some_and(|e| e.digest == digest)
        {
            return Ok(());
        }
    }
    let image = DynamicImage::ImageRgba8(pixels);
    let directory = tempfile::Builder::new()
        .prefix("job-fan-")
        .tempdir_in(crate::jobs::results(app)?)
        .explain()?;
    let name = format!("Screenshot-{id:03}.png");
    let file = directory.path().join(&name);
    image.save(&file).explain()?;
    let mut icon = Cursor::new(Vec::new());
    image
        .thumbnail(192, 128)
        .write_to(&mut icon, ImageFormat::Png)
        .explain()?;
    let icon = icon.into_inner();
    let preview = Preview {
        id,
        name,
        thumbnail: format!("data:image/png;base64,{}", STANDARD.encode(&icon)),
    };
    let mut history = state.lock().explain()?;
    if history.generation != generation
        || history
            .entries
            .iter()
            .any(|e| e.digest == digest && e.preview.id > id)
    {
        return Ok(());
    }
    history.entries.retain(|e| e.digest != digest);
    let index = history.entries.partition_point(|e| e.preview.id > id);
    history.entries.insert(
        index,
        Arc::new(Entry {
            preview,
            digest,
            file,
            _directory: directory,
        }),
    );
    history.entries.truncate(CAPACITY);
    app.emit("screenshot-fan-changed", history.previews())
        .explain()
}

#[derive(Deserialize)]
pub struct CardDragImage {
    png: String,
    x: i32,
    y: i32,
}

#[cfg(windows)]
mod windows_drag;

// A synchronous Tauri command runs on the UI thread, where OLE/GTK own the drag loop.
#[tauri::command]
pub fn drag_screenshot(window: WebviewWindow, id: u64, image: CardDragImage) -> AppResult<()> {
    let state = window.state::<Mutex<History>>();
    let entry = state
        .lock()
        .explain()?
        .entries
        .iter()
        .find(|e| e.preview.id == id)
        .cloned()
        .ok_or("Screenshot unavailable")?;
    if image.png.len() > 6_000_000 {
        return Err("Drag image too large".into());
    }
    let bytes = STANDARD.decode(image.png).explain()?;
    let (width, height) = image::ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png)
        .into_dimensions()
        .explain()?;
    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return Err("Invalid drag image dimensions".into());
    }
    let x = image.x.clamp(0, width as i32 - 1);
    let y = image.y.clamp(0, height as i32 - 1);
    #[cfg(windows)]
    {
        let pixels = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
            .explain()?
            .into_rgba8();
        let mut transparent = Cursor::new(Vec::new());
        RgbaImage::new(1, 1)
            .write_to(&mut transparent, ImageFormat::Png)
            .explain()?;
        let card = windows_drag::LiftedCard::show(pixels, x, y).explain()?;
        let result = drag::start_drag(
            &window,
            drag::DragItem::Files(vec![entry.file.clone()]),
            drag::Image::Raw(transparent.into_inner()),
            |_, _| {},
            drag::Options::default(),
        );
        drop(card);
        result.explain()?;
        window.emit("screenshot-drag-ended", ()).explain()
    }
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        use std::{cell::RefCell, rc::Rc};
        let handle = window.gtk_window().explain()?;
        let icon = gtk::gdk_pixbuf::Pixbuf::from_read(Cursor::new(bytes.clone())).explain()?;
        // GTK creates the context synchronously at drag-begin. Use it to anchor
        // the icon at the original grip after drag-rs sets its default icon.
        let context = Rc::new(RefCell::new(None));
        let started = context.clone();
        let handler =
            handle.connect_drag_begin(move |_, ctx| *started.borrow_mut() = Some(ctx.clone()));
        let result = drag::start_drag(
            &handle,
            drag::DragItem::Files(vec![entry.file.clone()]),
            drag::Image::Raw(bytes),
            move |_, _| {
                let _keep_alive = &entry;
                let _ = window.emit("screenshot-drag-ended", ());
            },
            drag::Options::default(),
        );
        handle.disconnect(handler);
        if let Some(ctx) = context.take() {
            ctx.drag_set_icon_pixbuf(&icon, x, y);
        }
        result.explain()
    }
}
