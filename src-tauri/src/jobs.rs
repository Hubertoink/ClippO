use crate::engine::{self, AppResult, Explain, Options};
use arboard::{Clipboard, ImageData};
use serde::Serialize;
#[cfg(windows)]
use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt};
#[cfg(windows)]
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    hash::{DefaultHasher, Hasher},
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Mutex},
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tempfile::{tempdir, NamedTempFile};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
#[cfg(windows)]
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

#[derive(Default)]
pub struct Session {
    jobs: Mutex<HashMap<String, Job>>,
    #[cfg(windows)]
    pub clipboard_sequence: AtomicU32,
    #[cfg(target_os = "linux")]
    clipboard_owner: Mutex<Option<Clipboard>>,
    pub watching: AtomicBool,
    pub ready: AtomicBool,
    pub docked: AtomicBool,
    pub collapsed: AtomicBool,
    pub teardrop_masked: AtomicBool,
    pub shortcut_failed: AtomicBool,
    pub ignored_path: Mutex<Option<(PathBuf, SystemTime)>>,
}
#[derive(Clone)]
struct Job {
    output: PathBuf,
    original: Option<PathBuf>,
    capture: Option<PathBuf>,
    digest: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    id: String,
    name: String,
    original: u64,
    size: u64,
    can_replace: bool,
    target_met: bool,
    saved: Option<String>,
    saved_path: Option<PathBuf>,
}
fn hash_file(file: &mut fs::File) -> AppResult<u64> {
    let mut hash = DefaultHasher::new();
    let mut buf = [0; 65536];
    loop {
        let n = file.read(&mut buf).explain()?;
        if n == 0 {
            break;
        }
        hash.write(&buf[..n])
    }
    Ok(hash.finish())
}
fn engines(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app
        .path()
        .resource_dir()
        .explain()?
        .join("resources/engines"))
}
pub(crate) fn results(app: &AppHandle) -> AppResult<PathBuf> {
    let p = app.path().app_cache_dir().explain()?.join("results");
    fs::create_dir_all(&p).explain()?;
    Ok(p)
}
pub fn clear_old(app: &AppHandle) {
    if let Ok(dir) = results(app) {
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                if e.file_name().to_string_lossy().starts_with("job-")
                    && e.file_type().is_ok_and(|t| t.is_dir())
                    && e.metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age > Duration::from_secs(86400))
                {
                    let _ = fs::remove_dir_all(e.path());
                }
            }
        }
    }
}
#[tauri::command]
pub async fn image_dimensions(app: AppHandle, path: PathBuf) -> AppResult<(u32, u32)> {
    tauri::async_runtime::spawn_blocking(move || engine::image_dimensions(&path, &engines(&app)?))
        .await
        .explain()?
}
#[tauri::command]
pub fn file_size(path: PathBuf) -> AppResult<u64> {
    let meta = fs::metadata(path).explain()?;
    if !meta.is_file() {
        return Err("Select a file.".into());
    }
    Ok(meta.len())
}
fn save_copy(job: &Job, dir: &Path) -> AppResult<PathBuf> {
    if !dir.is_dir() {
        return Err(format!("Folder unavailable: {}", dir.display()));
    }
    let mut tmp = NamedTempFile::new_in(dir).explain()?;
    std::io::copy(&mut fs::File::open(&job.output).explain()?, &mut tmp).explain()?;
    tmp.as_file().sync_all().explain()?;
    for n in 1..10000 {
        let name = if n == 1 {
            job.output.file_name().unwrap().to_owned()
        } else {
            format!(
                "{} ({n}).{}",
                job.output.file_stem().unwrap().to_string_lossy(),
                engine::ext(&job.output)
            )
            .into()
        };
        let path = dir.join(name);
        match tmp.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => tmp = e.file,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Choose another destination folder.".into())
}
#[cfg(windows)]
fn replace(job: &Job) -> AppResult<()> {
    let path = job.original.as_ref().ok_or("No original file on disk.")?;
    if engine::ext(path) != engine::ext(&job.output) {
        return Err("Save the converted format as a copy.".into());
    }
    let mut original = fs::OpenOptions::new()
        .read(true)
        .share_mode(1 | 4)
        .open(path)
        .explain()?;
    if hash_file(&mut original)? != job.digest {
        return Err("The original changed after compression. Save a copy.".into());
    }
    let staging = tempfile::Builder::new()
        .prefix(".clippo-")
        .tempdir_in(path.parent().ok_or("Missing parent folder.")?)
        .explain()?;
    let replacement = staging.path().join("optimized");
    let backup = staging.path().join("original");
    fs::copy(&job.output, &replacement).explain()?;
    fs::OpenOptions::new()
        .write(true)
        .open(&replacement)
        .explain()?
        .sync_all()
        .explain()?;
    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<u16>>()
    };
    // ReplaceFile retains the destination's ACLs and streams. Keep the verified handle
    // open to exclude writers, and retain a recovery copy if Windows reports a partial move.
    let ok = unsafe {
        ReplaceFileW(
            wide(path).as_ptr(),
            wide(&replacement).as_ptr(),
            wide(&backup).as_ptr(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        if backup.exists() {
            let retained = staging.keep();
            return Err(format!(
                "{error}. Recovery copy: {}",
                retained.join("original").display()
            ));
        }
        return Err(error.to_string());
    }
    drop(original);
    Ok(())
}
#[cfg(target_os = "linux")]
fn replace(job: &Job) -> AppResult<()> {
    use std::os::unix::fs::MetadataExt;
    let path = job.original.as_ref().ok_or("No original file on disk.")?;
    if engine::ext(path) != engine::ext(&job.output) {
        return Err("Save the converted format as a copy.".into());
    }
    let mut original = fs::File::open(path).explain()?;
    original.try_lock().explain()?;
    let meta = original.metadata().explain()?;
    let mut tmp =
        NamedTempFile::new_in(path.parent().ok_or("Missing parent folder.")?).explain()?;
    std::io::copy(&mut fs::File::open(&job.output).explain()?, &mut tmp).explain()?;
    tmp.as_file()
        .set_permissions(meta.permissions())
        .explain()?;
    tmp.as_file().sync_all().explain()?;
    let current = fs::symlink_metadata(path).explain()?;
    if !current.is_file()
        || current.ino() != meta.ino()
        || current.dev() != meta.dev()
        || hash_file(&mut original)? != job.digest
    {
        return Err("The original changed after compression. Save a copy.".into());
    }
    tmp.persist(path).explain()?;
    Ok(())
}
fn process(
    app: AppHandle,
    source: Option<PathBuf>,
    capture: Option<PathBuf>,
    options: Options,
) -> AppResult<Output> {
    let scratch = tempdir().explain()?;
    let (input, name) = if let Some(path) = source.as_ref().or(capture.as_ref()) {
        if !engine::supported(path) {
            return Err("Unsupported file format.".into());
        }
        let size = file_size(path.clone())?;
        if size == 0 || size > 256 * 1024 * 1024 {
            return Err("Choose a non-empty file up to 256 MB.".into());
        }
        let input = scratch.path().join(format!("source.{}", engine::ext(path)));
        fs::copy(path, &input).explain()?;
        (
            input,
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        let image = Clipboard::new()
            .explain()?
            .get_image()
            .map_err(|_| "Copy an image or a file first.")?;
        let input = scratch.path().join("source.png");
        image::save_buffer(
            &input,
            &image.bytes,
            image.width as u32,
            image.height as u32,
            image::ColorType::Rgba8,
        )
        .explain()?;
        // A screenshot pasted after startup (or after a watcher miss) still belongs
        // in the fan. The history deduplicates the watcher's copy by pixel content.
        let _ = crate::screenshot_fan::remember(&app, Some(&input));
        (input, "Screenshot".to_owned())
    };
    let digest = hash_file(&mut fs::File::open(&input).explain()?)?;
    let original = fs::metadata(&input).explain()?.len();
    let progress = |size| {
        let _ = app.emit("compression-progress", size);
    };
    let compressed =
        engine::compress(&input, scratch.path(), &engines(&app)?, &options, &progress)?;
    let size = fs::metadata(&compressed).explain()?.len();
    let name = format!("{name}-clippo.{}", engine::ext(&compressed));
    let dir = tempfile::Builder::new()
        .prefix("job-")
        .tempdir_in(results(&app)?)
        .explain()?;
    let id = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let path = dir.path().join(&name);
    fs::copy(&compressed, &path).explain()?;
    let capture = if source.is_none() {
        if capture.is_some() {
            capture
        } else {
            let path = dir.path().join("Screenshot.png");
            fs::copy(&input, &path).explain()?;
            Some(path)
        }
    } else {
        None
    };
    let _ = dir.keep();
    let mut can_replace = source
        .as_ref()
        .is_some_and(|p| engine::ext(p) == engine::ext(&path))
        && size < original;
    let job = Job {
        output: path,
        original: source,
        capture,
        digest,
    };
    let state = app.state::<Session>();
    state.jobs.lock().explain()?.insert(id.clone(), job.clone());
    let mut saved = None;
    let mut saved_path = None;
    let de = options.language == "de";
    if !options.folder.is_empty() {
        saved = Some(match save_copy(&job, Path::new(&options.folder)) {
            Ok(p) => {
                saved_path = Some(p.clone());
                format!(
                    "{} {}",
                    if de { "Gespeichert in" } else { "Saved to" },
                    p.display()
                )
            }
            Err(e) => e,
        })
    }
    if options.replace && can_replace {
        saved_path = None;
        *state.ignored_path.lock().explain()? =
            job.original.clone().map(|p| (p, SystemTime::now()));
        saved = Some(match replace(&job) {
            Ok(_) => {
                can_replace = false;
                if de {
                    "Original ersetzt"
                } else {
                    "Original replaced"
                }
                .into()
            }
            Err(e) => e,
        })
    }
    Ok(Output {
        id,
        name,
        original,
        size,
        can_replace,
        target_met: size <= options.target(),
        saved,
        saved_path,
    })
}
#[tauri::command]
pub async fn compress_file(app: AppHandle, path: PathBuf, options: Options) -> AppResult<Output> {
    tauri::async_runtime::spawn_blocking(move || process(app, Some(path), None, options))
        .await
        .explain()?
}
#[tauri::command]
pub async fn compress_clipboard(app: AppHandle, options: Options) -> AppResult<Output> {
    tauri::async_runtime::spawn_blocking(move || {
        let file = Clipboard::new()
            .explain()?
            .get()
            .file_list()
            .ok()
            .and_then(|ps| ps.into_iter().find(|p| engine::supported(p)));
        process(app, file, None, options)
    })
    .await
    .explain()?
}
#[tauri::command]
pub async fn compress_capture(app: AppHandle, id: String, options: Options) -> AppResult<Output> {
    tauri::async_runtime::spawn_blocking(move || {
        let job = app
            .state::<Session>()
            .jobs
            .lock()
            .explain()?
            .get(&id)
            .cloned()
            .ok_or("Capture expired.")?;
        process(app, job.original, job.capture, options)
    })
    .await
    .explain()?
}
#[cfg(windows)]
fn classic_outlook() -> Option<PathBuf> {
    for root in ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
    {
        for sub in [
            "Microsoft Office/root/Office16/OUTLOOK.EXE",
            "Microsoft Office/Office16/OUTLOOK.EXE",
        ] {
            let path = PathBuf::from(&root).join(sub);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}
fn remember_clipboard(state: &Session, _clipboard: Clipboard) -> AppResult<()> {
    #[cfg(windows)]
    state
        .clipboard_sequence
        .store(unsafe { GetClipboardSequenceNumber() }, Ordering::Relaxed);
    #[cfg(target_os = "linux")]
    {
        *state.clipboard_owner.lock().explain()? = Some(_clipboard);
    }
    Ok(())
}
fn act(
    app: AppHandle,
    id: String,
    action: String,
    folder: String,
    language: String,
) -> AppResult<String> {
    let state = app.state::<Session>();
    let job = state
        .jobs
        .lock()
        .explain()?
        .get(&id)
        .cloned()
        .ok_or("This result has expired.")?;
    let de = language == "de";
    let tr = |en: &str, ger: &str| if de { ger.to_string() } else { en.to_string() };
    let cb = || -> AppResult<()> {
        let mut clipboard = Clipboard::new().explain()?;
        clipboard.set().file_list(&[&job.output]).explain()?;
        remember_clipboard(&state, clipboard)?;
        Ok(())
    };
    match action.as_str() {
        "file" => {
            cb()?;
            Ok(tr("File copied", "Datei kopiert"))
        }
        "copy" => {
            let tmp = tempdir().explain()?;
            let image = engine::as_png(&engines(&app)?, &job.output, tmp.path())?.to_rgba8();
            let mut clipboard = Clipboard::new().explain()?;
            clipboard
                .set_image(ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: Cow::Owned(image.into_raw()),
                })
                .explain()?;
            remember_clipboard(&state, clipboard)?;
            Ok(tr("Image copied", "Bild kopiert"))
        }
        "save" | "save-path" => {
            let dest = if !folder.is_empty() {
                PathBuf::from(folder)
            } else if let Some(p) = job.original.as_ref().and_then(|p| p.parent()) {
                p.to_owned()
            } else {
                app.path().download_dir().explain()?
            };
            let path = save_copy(&job, &dest)?;
            if action == "save-path" {
                return Ok(path.to_string_lossy().into_owned());
            }
            Ok(format!(
                "{} {}",
                tr("Saved to", "Gespeichert in"),
                path.display()
            ))
        }
        "replace" => {
            *state.ignored_path.lock().explain()? =
                job.original.clone().map(|p| (p, SystemTime::now()));
            replace(&job)?;
            Ok(tr("Original replaced", "Original ersetzt"))
        }
        "attachment" => {
            app.opener().reveal_item_in_dir(&job.output).explain()?;
            Ok(tr(
                "Attachment shown in file manager",
                "Anhang im Dateimanager angezeigt",
            ))
        }
        #[cfg(windows)]
        "teams" => {
            cb()?;
            app.opener().open_url("msteams:", None::<&str>).explain()?;
            Ok(tr(
                "Paste the file into your Teams chat",
                "Datei jetzt im Teams-Chat einfügen",
            ))
        }
        #[cfg(windows)]
        "outlook" => {
            if let Some(exe) = classic_outlook() {
                engine::hidden(&exe)
                    .arg("/c")
                    .arg("ipm.note")
                    .arg("/a")
                    .arg(&job.output)
                    .spawn()
                    .explain()?;
                Ok(tr("Outlook draft opened", "Outlook-Entwurf geöffnet"))
            } else {
                cb()?;
                app.opener()
                    .open_url("ms-outlook://compose", None::<&str>)
                    .explain()?;
                Ok(tr(
                    "Paste the copied file into the Outlook draft",
                    "Kopierte Datei in den Outlook-Entwurf einfügen",
                ))
            }
        }
        _ => Err("Unknown action.".into()),
    }
}
#[tauri::command]
pub async fn reveal_saved(app: AppHandle, path: PathBuf) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || app.opener().reveal_item_in_dir(path).explain())
        .await
        .explain()?
}
#[tauri::command]
pub async fn result_action(
    app: AppHandle,
    id: String,
    action: String,
    folder: String,
    language: String,
) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || act(app, id, action, folder, language))
        .await
        .explain()?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(dir: &Path) -> Job {
        let original = dir.join("document.pdf");
        let output = dir.join("document-clippo.pdf");
        fs::write(&original, b"original contents").unwrap();
        fs::write(&output, b"smaller").unwrap();
        let digest = hash_file(&mut fs::File::open(&original).unwrap()).unwrap();
        Job {
            original: Some(original),
            output,
            digest,
            capture: None,
        }
    }
    #[test]
    fn replaces_verified_original_and_refuses_later_changes() {
        let dir = tempdir().unwrap();
        let job = fixture(dir.path());
        replace(&job).unwrap();
        let path = job.original.as_ref().unwrap();
        assert_eq!(fs::read(path).unwrap(), b"smaller");
        fs::write(path, b"edited by another application").unwrap();
        assert!(replace(&job).is_err());
        assert_eq!(fs::read(path).unwrap(), b"edited by another application");
    }
    #[test]
    fn repeated_saves_get_distinct_names() {
        let dir = tempdir().unwrap();
        let job = fixture(dir.path());
        let first = save_copy(&job, dir.path()).unwrap();
        let next = save_copy(&job, dir.path()).unwrap();
        assert_ne!(first, next);
        assert_eq!(fs::read(first).unwrap(), b"smaller");
        assert_eq!(fs::read(next).unwrap(), b"smaller");
    }
}
