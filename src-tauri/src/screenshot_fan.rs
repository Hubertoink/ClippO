use crate::engine::{AppResult, Explain};
use arboard::Clipboard;
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::Serialize;
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
    icon: Vec<u8>,
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
            icon,
            _directory: directory,
        }),
    );
    history.entries.truncate(CAPACITY);
    app.emit("screenshot-fan-changed", history.previews())
        .explain()
}

// A synchronous Tauri command runs on the UI thread, where OLE/GTK own the drag loop.
#[tauri::command]
pub fn drag_screenshot(window: WebviewWindow, id: u64) -> AppResult<()> {
    let state = window.state::<Mutex<History>>();
    let entry = state
        .lock()
        .explain()?
        .entries
        .iter()
        .find(|e| e.preview.id == id)
        .cloned()
        .ok_or("Screenshot unavailable")?;
    #[cfg(target_os = "linux")]
    let gtk_window = window.gtk_window().explain()?;
    #[cfg(target_os = "linux")]
    let handle = &gtk_window;
    #[cfg(not(target_os = "linux"))]
    let handle = &window;
    let finished = window.clone();
    drag::start_drag(
        handle,
        drag::DragItem::Files(vec![entry.file.clone()]),
        drag::Image::Raw(entry.icon.clone()),
        move |_, _| {
            // Retain the full image if it rolls out of history during a drag.
            let _keep_alive = &entry;
            let _ = finished.emit("screenshot-drag-ended", ());
        },
        drag::Options::default(),
    )
    .explain()
}
