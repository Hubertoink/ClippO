use crate::engine::{self, AppResult, Explain, Options};
use serde::Serialize;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultEvent {
    kind: &'static str,
    name: String,
    original: u64,
    size: u64,
    path: PathBuf,
}

#[derive(Serialize)]
struct ErrorEvent {
    kind: &'static str,
    path: String,
    message: String,
}

pub fn compress(arguments: &[String]) -> i32 {
    let parsed = parse(arguments);
    let (paths, options, folder) = match parsed {
        Ok(parsed) => parsed,
        Err(message) => {
            emit_error("", message);
            return 2;
        }
    };
    if paths.is_empty() {
        emit_error("", "Choose at least one image, PDF or WAV file.".into());
        return 2;
    }
    let mut failed = false;
    for path in paths {
        match compress_one(&path, &options, folder.as_deref()) {
            Ok(result) => println!("{}", serde_json::to_string(&result).unwrap()),
            Err(message) => {
                failed = true;
                emit_error(&path.to_string_lossy(), message);
            }
        }
    }
    if failed {
        1
    } else {
        0
    }
}

fn parse(arguments: &[String]) -> AppResult<(Vec<PathBuf>, Options, Option<PathBuf>)> {
    let mut mode = "Balanced".to_owned();
    let mut output = "Original".to_owned();
    let mut folder = None;
    let mut under_mb = false;
    let mut preserve = false;
    let mut paths = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        let value = if matches!(
            argument.as_str(),
            "--mode" | "--output" | "--folder" | "--resize-percent"
        ) {
            index += 1;
            arguments
                .get(index)
                .ok_or_else(|| format!("Missing value for {argument}."))?
        } else {
            argument
        };
        match argument.as_str() {
            "--mode" => mode = value.clone(),
            "--output" => output = value.clone(),
            "--folder" => folder = Some(PathBuf::from(value)),
            "--under-mb" => under_mb = true,
            "--preserve" => preserve = true,
            "--resize-percent" => {
                return Err("Resizing is not available from the panel yet.".into());
            }
            _ if argument.starts_with("--") => return Err(format!("Unknown option {argument}.")),
            _ => paths.push(PathBuf::from(argument.trim_start_matches("file://"))),
        }
        index += 1;
    }
    if !matches!(mode.as_str(), "Smart" | "Balanced" | "Maximum") {
        return Err("Choose Smart, Balanced or Maximum compression.".into());
    }
    if !matches!(output.as_str(), "Original" | "WebP" | "JPEG" | "PNG") {
        return Err("Choose a supported output format.".into());
    }
    Ok((
        paths,
        Options {
            mode,
            under_mb,
            max_kb: 0,
            resize_percent: None,
            jpg: 80,
            webp: 78,
            png: 2,
            pdf: 72,
            mp3_kbps: Some(192),
            output,
            preserve,
            folder: String::new(),
            replace: false,
            language: "en".into(),
        },
        folder,
    ))
}

fn compress_one(
    source: &Path,
    options: &Options,
    requested_folder: Option<&Path>,
) -> AppResult<ResultEvent> {
    if !engine::supported(source) {
        return Err("Unsupported file format.".into());
    }
    let original = fs::metadata(source).explain()?.len();
    if original == 0 || original > 256 * 1024 * 1024 {
        return Err("Choose a non-empty file up to 256 MB.".into());
    }
    let work = tempdir().explain()?;
    let compressed = engine::compress(source, work.path(), Path::new(""), options, &|_| {})?;
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Clip")
        .to_owned();
    let extension = engine::ext(&compressed);
    let destination = requested_folder
        .map(Path::to_path_buf)
        .or_else(|| source.parent().map(Path::to_path_buf))
        .ok_or("The output folder is unavailable.")?;
    fs::create_dir_all(&destination).explain()?;
    let path = unique_path(&destination, &stem, &extension);
    fs::copy(&compressed, &path).explain()?;
    Ok(ResultEvent {
        kind: "result",
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        original,
        size: fs::metadata(&path).explain()?.len(),
        path,
    })
}

fn unique_path(folder: &Path, stem: &str, extension: &str) -> PathBuf {
    let initial = folder.join(format!("{stem}-clippo.{extension}"));
    if !initial.exists() {
        return initial;
    }
    for index in 2..10_000 {
        let candidate = folder.join(format!("{stem}-clippo-{index}.{extension}"));
        if let Err(error) = fs::metadata(&candidate) {
            if error.kind() == ErrorKind::NotFound {
                return candidate;
            }
        }
    }
    initial.with_file_name(format!("{stem}-clippo-latest.{extension}"))
}

fn emit_error(path: &str, message: String) {
    let event = ErrorEvent {
        kind: "error",
        path: path.to_owned(),
        message,
    };
    eprintln!("{}", serde_json::to_string(&event).unwrap());
}
