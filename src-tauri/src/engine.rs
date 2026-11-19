use image::{DynamicImage, ImageDecoder, ImageReader};
use serde::Deserialize;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

pub type AppResult<T> = Result<T, String>;
pub trait Explain<T> {
    fn explain(self) -> AppResult<T>;
}
impl<T, E: std::fmt::Display> Explain<T> for Result<T, E> {
    fn explain(self) -> AppResult<T> {
        self.map_err(|e| e.to_string())
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub mode: String,
    pub under_mb: bool,
    pub max_kb: u64,
    #[serde(default)]
    pub resize_percent: Option<u32>,
    pub jpg: u8,
    pub webp: u8,
    pub png: u8,
    pub pdf: u8,
    pub output: String,
    pub preserve: bool,
    pub folder: String,
    pub replace: bool,
    pub language: String,
}
impl Options {
    pub fn target(&self) -> u64 {
        let ceiling = if self.max_kb > 0 {
            self.max_kb.saturating_mul(1000)
        } else {
            u64::MAX
        };
        if self.under_mb {
            ceiling.min(999_999)
        } else {
            ceiling
        }
    }
    fn quality(&self, ext: &str) -> u8 {
        match self.mode.as_str() {
            "Smart" => 90,
            "Maximum" => 48,
            "Custom" => match ext {
                "webp" => self.webp,
                "pdf" => self.pdf,
                _ => self.jpg,
            }
            .clamp(10, 100),
            _ => 78,
        }
    }
}
pub fn ext(path: &Path) -> String {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "jpeg" => "jpg".into(),
        "heif" => "heic".into(),
        s => s.to_string(),
    }
}
pub fn supported(p: &Path) -> bool {
    matches!(
        ext(p).as_str(),
        "png" | "jpg" | "webp" | "avif" | "heic" | "pdf"
    )
}
pub fn hidden(exe: &Path) -> Command {
    let mut command = Command::new(exe);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x08000000);
    command
}
fn execute(command: &mut Command, qpdf: bool) -> AppResult<()> {
    let r = command.output().explain()?;
    if r.status.success() || qpdf && r.status.code() == Some(3) {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&r.stderr)
        .trim()
        .chars()
        .take(400)
        .collect())
}
fn jpeg(image: &DynamicImage, quality: u8) -> AppResult<Vec<u8>> {
    let rgb = if image.color().has_alpha() {
        let mut matte = image::RgbaImage::from_pixel(
            image.width(),
            image.height(),
            image::Rgba([255, 255, 255, 255]),
        );
        image::imageops::overlay(&mut matte, &image.to_rgba8(), 0, 0);
        DynamicImage::ImageRgba8(matte).to_rgb8()
    } else {
        image.to_rgb8()
    };
    let mut encoder = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    encoder.set_size(rgb.width() as usize, rgb.height() as usize);
    encoder.set_quality(quality.into());
    let mut out = encoder.start_compress(Vec::new()).explain()?;
    out.write_scanlines(&rgb).explain()?;
    out.finish().explain()
}
fn image_tool(_engines: &Path, identify: bool) -> Command {
    #[cfg(windows)]
    {
        let mut command = hidden(&_engines.join("magick/magick.exe"));
        if identify {
            command.arg("identify");
        }
        command
    }
    #[cfg(target_os = "linux")]
    {
        Command::new(if identify {
            "/usr/bin/identify"
        } else {
            "/usr/bin/convert"
        })
    }
}
pub fn image_dimensions(input: &Path, engines: &Path) -> AppResult<(u32, u32)> {
    if !supported(input) || ext(input) == "pdf" {
        return Err("Expected an image.".into());
    }
    if matches!(ext(input).as_str(), "jpg" | "png" | "webp") {
        let mut decoder = ImageReader::open(input)
            .explain()?
            .with_guessed_format()
            .explain()?
            .into_decoder()
            .explain()?;
        let (w, h) = decoder.dimensions();
        return Ok(if decoder.orientation().explain()?.to_exif() >= 5 {
            (h, w)
        } else {
            (w, h)
        });
    }
    let output = image_tool(engines, true)
        .args(["-ping", "-format", "%w %h %[orientation]"])
        .arg(format!("{}[0]", input.display()))
        .output()
        .explain()?;
    if !output.status.success() {
        return Err("Cannot read image dimensions.".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = text.split_whitespace();
    let mut dimension = || {
        fields
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n > 0)
            .ok_or("Invalid image dimensions.")
    };
    let (w, h) = (dimension()?, dimension()?);
    Ok(
        if matches!(
            fields.next(),
            Some("LeftTop" | "RightTop" | "RightBottom" | "LeftBottom")
        ) {
            (h, w)
        } else {
            (w, h)
        },
    )
}
fn decode(input: &Path) -> AppResult<DynamicImage> {
    let mut decoder = ImageReader::open(input)
        .explain()?
        .with_guessed_format()
        .explain()?
        .into_decoder()
        .explain()?;
    if u64::from(decoder.dimensions().0) * u64::from(decoder.dimensions().1) > 80_000_000 {
        return Err("Image exceeds 80 megapixels.".into());
    }
    let orientation = decoder.orientation().explain()?;
    let mut image = DynamicImage::from_decoder(decoder).explain()?;
    image.apply_orientation(orientation);
    Ok(image)
}
pub fn as_png(engines: &Path, path: &Path, temp: &Path) -> AppResult<DynamicImage> {
    if let Ok(image) = decode(path) {
        return Ok(image);
    }
    let dest = temp.join("clipboard.png");
    magick(engines, path, &dest, 100, 100, 0, false)?;
    decode(&dest)
}
fn magick(
    engines: &Path,
    input: &Path,
    out: &Path,
    quality: u8,
    scale: u32,
    colors: u32,
    preserve: bool,
) -> AppResult<()> {
    let mut c = image_tool(engines, false);
    c.args([
        "-limit", "memory", "256MiB", "-limit", "map", "512MiB", "-limit", "disk", "1GiB",
        "-limit", "time", "90",
    ])
    .arg(input)
    .arg("-auto-orient");
    if scale < 100 {
        c.args(["-resize", &format!("{scale}%")]);
    }
    if !preserve {
        c.arg("-strip");
    }
    if ext(out) == "jpg" {
        c.args(["-background", "white", "-alpha", "remove", "-alpha", "off"]);
    }
    if colors > 0 {
        c.args(["+dither", "-colors", &colors.to_string()]);
    }
    c.args(["-quality", &quality.to_string()]).arg(out);
    execute(&mut c, false)
}
fn png_options(s: &Options) -> oxipng::Options {
    let preset = match s.mode.as_str() {
        "Custom" => s.png.min(6),
        "Maximum" => 3,
        "Smart" => 1,
        _ => 1,
    };
    let mut opts = oxipng::Options::from_preset(preset);
    opts.strip = if s.preserve {
        oxipng::StripChunks::None
    } else {
        oxipng::StripChunks::Safe
    };
    opts
}
fn images(
    input: &Path,
    work: &Path,
    engines: &Path,
    s: &Options,
    progress: &dyn Fn(u64),
) -> AppResult<PathBuf> {
    let original_ext = ext(input);
    let animated = match original_ext.as_str() {
        "webp" => image::codecs::webp::WebPDecoder::new(std::io::BufReader::new(
            fs::File::open(input).explain()?,
        ))
        .explain()?
        .has_animation(),
        "png" => image::codecs::png::PngDecoder::new(std::io::BufReader::new(
            fs::File::open(input).explain()?,
        ))
        .explain()?
        .is_apng()
        .explain()?,
        _ => false,
    };
    if animated {
        return Err(if s.language == "de" {
            "Bitte ein Einzelbild auswählen. Diese Datei enthält mehrere Animationsframes."
        } else {
            "Choose a still image. This file contains animation frames."
        }
        .into());
    }
    let output_ext = match s.output.as_str() {
        "WebP" => "webp",
        "JPEG" => "jpg",
        "PNG" => "png",
        _ if original_ext == "heic" => "webp",
        _ => &original_ext,
    };
    let mut best = input.to_path_buf();
    let mut best_size = if original_ext == output_ext && s.resize_percent.is_none() {
        fs::metadata(input).explain()?.len()
    } else {
        u64::MAX
    };
    let use_native = !s.preserve
        && matches!(original_ext.as_str(), "png" | "jpg" | "webp")
        && matches!(output_ext, "jpg" | "webp");
    let decoded = if use_native {
        Some(decode(input)?)
    } else {
        None
    };
    let mut scale = s.resize_percent.unwrap_or(100).clamp(1, 100);
    for pass in 0..8u8 {
        let dest = work.join(format!("encoded-{pass}.{output_ext}"));
        let quality = s.quality(output_ext).saturating_sub(pass * 8).max(16);
        if let Some(image) = &decoded {
            let resized;
            let img = if scale == 100 {
                image
            } else {
                resized = image.resize_exact(
                    ((image.width() as u64 * scale as u64 + 50) / 100).max(1) as u32,
                    ((image.height() as u64 * scale as u64 + 50) / 100).max(1) as u32,
                    image::imageops::FilterType::Triangle,
                );
                &resized
            };
            let bytes = if output_ext == "jpg" {
                jpeg(img, quality)?
            } else {
                webp::Encoder::from_rgba(&img.to_rgba8(), img.width(), img.height())
                    .encode(quality.into())
                    .to_vec()
            };
            fs::write(&dest, bytes).explain()?;
        } else if output_ext == "png"
            && original_ext == "png"
            && scale == 100
            && matches!(s.mode.as_str(), "Smart" | "Custom")
        {
            let data = fs::read(input).explain()?;
            let optimized = oxipng::optimize_from_memory(&data, &png_options(s)).explain()?;
            fs::write(&dest, optimized).explain()?;
        } else {
            let colors = if output_ext == "png" && !s.preserve && s.mode != "Smart" {
                if s.mode == "Maximum" {
                    96
                } else {
                    256
                }
            } else {
                0
            };
            magick(engines, input, &dest, quality, scale, colors, s.preserve)?;
        }
        let size = fs::metadata(&dest).explain()?.len();
        if size == 0 {
            return Err("Encoder produced an empty file.".into());
        }
        if size < best_size {
            best = dest;
            best_size = size
        }
        progress(best_size);
        if best_size <= s.target() {
            break;
        }
        let ratio = ((s.target() as f64 / best_size as f64).sqrt() * 0.95).clamp(0.35, 0.85);
        scale = ((scale as f64 * ratio).round() as u32).max(1);
    }
    Ok(best)
}
#[cfg(target_os = "linux")]
fn ghostscript() -> Option<PathBuf> {
    let gs = PathBuf::from("/usr/bin/gs");
    gs.is_file().then_some(gs)
}
#[cfg(windows)]
fn ghostscript() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("ProgramFiles")?).join("gs");
    let mut paths: Vec<_> = fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path().join("bin/gswin64c.exe"))
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    paths.pop()
}
// Keep the four ink channels and the JPEG color transform used by the PDF.
fn recompress_cmyk(data: &[u8], quality: u8) -> std::io::Result<Vec<u8>> {
    use mozjpeg::{ColorSpace, Compress, Decompress};
    let decoder = Decompress::new_mem(data)?;
    let space = decoder.color_space();
    if !matches!(space, ColorSpace::JCS_CMYK | ColorSpace::JCS_YCCK)
        || decoder.width().saturating_mul(decoder.height()) > 40_000_000
    {
        return Err(std::io::Error::other("Unsupported four-channel JPEG"));
    }
    let mut raw = decoder.to_colorspace(ColorSpace::JCS_CMYK)?;
    let mut enc = Compress::new(ColorSpace::JCS_CMYK);
    enc.set_color_space(space);
    enc.set_size(raw.width(), raw.height());
    enc.set_quality(quality.into());
    let mut out = enc.start_compress(Vec::new())?;
    out.write_scanlines(&raw.read_scanlines::<u8>()?)?;
    raw.finish()?;
    out.finish()
}
fn reduce_pdf_images(doc: &mut lopdf::Document, quality: u8, edge: u32) -> AppResult<()> {
    // Only independent RGB JPEGs are safe to resize without rewriting associated masks.
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else {
            continue;
        };
        let d = &stream.dict;
        if d.get(b"Filter")
            .and_then(|v| v.as_name())
            .is_ok_and(|n| n == b"DCTDecode")
            && d.get(b"ColorSpace")
                .and_then(|v| v.as_name())
                .is_ok_and(|n| n == b"DeviceCMYK")
            && d.get(b"Subtype")
                .and_then(|v| v.as_name())
                .is_ok_and(|n| n == b"Image")
            && !d.has(b"DecodeParms")
        {
            // Dimensions stay identical, including for separately referenced soft masks.
            if let Ok(bytes) = recompress_cmyk(&stream.content, quality) {
                if bytes.len() < stream.content.len() {
                    stream.set_content(bytes);
                }
            }
            continue;
        }
        if !d
            .get(b"Subtype")
            .and_then(|v| v.as_name())
            .is_ok_and(|n| n == b"Image")
            || !d
                .get(b"Filter")
                .and_then(|v| v.as_name())
                .is_ok_and(|n| n == b"DCTDecode")
            || !d
                .get(b"ColorSpace")
                .and_then(|v| v.as_name())
                .is_ok_and(|n| n == b"DeviceRGB")
            || [b"SMask".as_slice(), b"Mask", b"Decode", b"DecodeParms"]
                .iter()
                .any(|k| d.has(k))
        {
            continue;
        }
        let reader =
            ImageReader::with_format(Cursor::new(&stream.content), image::ImageFormat::Jpeg);
        let Ok(image) = reader.decode() else { continue };
        let reduced = image.thumbnail(edge.min(image.width()), edge.min(image.height()));
        let bytes = jpeg(&reduced, quality)?;
        if bytes.len() < stream.content.len() {
            stream.dict.set("Width", i64::from(reduced.width()));
            stream.dict.set("Height", i64::from(reduced.height()));
            stream.set_content(bytes);
        }
    }
    Ok(())
}
fn pdf(
    input: &Path,
    work: &Path,
    engines: &Path,
    s: &Options,
    progress: &dyn Fn(u64),
) -> AppResult<PathBuf> {
    let doc = lopdf::Document::load(input).explain()?;
    if doc.is_encrypted() {
        return Err("Unlock the PDF before compression.".into());
    }
    if doc
        .objects
        .values()
        .any(|v| v.as_dict().is_ok_and(|d| d.has(b"ByteRange")))
    {
        return Err("This PDF contains a digital signature.".into());
    }
    let simple = doc.objects.values().all(|obj| {
        obj.as_dict()
            .map(|d| {
                ![
                    b"AcroForm".as_slice(),
                    b"Annots",
                    b"StructTreeRoot",
                    b"EmbeddedFiles",
                    b"OCProperties",
                    b"Outlines",
                    b"OutputIntents",
                ]
                .iter()
                .any(|k| d.has(k))
            })
            .unwrap_or(true)
    });
    let gs = if simple && !s.preserve {
        ghostscript()
    } else {
        None
    };
    let mut best = input.to_path_buf();
    for pass in 0..4u8 {
        let q = s.quality("pdf").saturating_sub(pass * 18).max(15);
        let mid = work.join(format!("prepared-{pass}.pdf"));
        if let Some(exe) = &gs {
            let dpi = 45 + q as u32 * 2;
            let mut c = hidden(exe);
            c.args([
                "-dSAFER",
                "-dBATCH",
                "-dNOPAUSE",
                "-dQUIET",
                "-sDEVICE=pdfwrite",
                "-dCompatibilityLevel=1.7",
                "-dDetectDuplicateImages=true",
                "-dSubsetFonts=true",
                "-dEmbedAllFonts=true",
                "-dDownsampleColorImages=true",
                "-dDownsampleGrayImages=true",
                "-dPassThroughJPEGImages=false",
                "-dColorImageDownsampleType=/Bicubic",
                "-dGrayImageDownsampleType=/Bicubic",
            ])
            .arg(format!("-dColorImageResolution={dpi}"))
            .arg(format!("-dGrayImageResolution={dpi}"))
            .arg(format!("-dJPEGQ={q}"))
            .arg(format!("-sOutputFile={}", mid.display()))
            .arg("-f")
            .arg(input);
            execute(&mut c, false)?;
        } else {
            let mut prepared = doc.clone();
            reduce_pdf_images(&mut prepared, q, 320 + u32::from(q) * 25)?;
            prepared.prune_objects();
            prepared.compress();
            prepared.save(&mid).explain()?;
        }
        let dest = work.join(format!("linearized-{pass}.pdf"));
        #[cfg(windows)]
        let mut c = hidden(&engines.join("qpdf/qpdf.exe"));
        #[cfg(target_os = "linux")]
        let mut c = {
            // Ubuntu 24.04 ships qpdf 11, predating its metadata removal switches.
            if !s.preserve {
                let mut document = lopdf::Document::load(&mid).explain()?;
                document.trailer.remove(b"Info");
                document.catalog_mut().explain()?.remove(b"Metadata");
                document.prune_objects();
                document.save(&mid).explain()?;
            }
            Command::new("/usr/bin/qpdf")
        };
        c.args([
            "--linearize",
            "--object-streams=generate",
            "--recompress-flate",
            "--compression-level=9",
            "--remove-unreferenced-resources=yes",
        ]);
        if gs.is_none() && !s.preserve {
            c.arg("--optimize-images");
            #[cfg(windows)]
            c.arg(format!("--jpeg-quality={q}"));
        }
        #[cfg(windows)]
        if !s.preserve {
            c.args(["--remove-info", "--remove-metadata"]);
        }
        c.arg(&mid).arg(&dest);
        execute(&mut c, true)?;
        if fs::metadata(&dest).explain()?.len() < fs::metadata(&best).explain()?.len() {
            best = dest
        }
        let best_size = fs::metadata(&best).explain()?.len();
        progress(best_size);
        if best_size <= s.target() {
            break;
        }
    }
    Ok(best)
}
pub fn compress(
    input: &Path,
    work: &Path,
    engines: &Path,
    s: &Options,
    progress: &dyn Fn(u64),
) -> AppResult<PathBuf> {
    if ext(input) == "pdf" {
        pdf(input, work, engines, s, progress)
    } else {
        images(input, work, engines, s, progress)
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;

    #[test]
    fn pixel_preview_matches_encoded_dimensions_for_odd_sizes() {
        let work = tempfile::tempdir().unwrap();
        let engines = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/engines");
        let options = Options {
            mode: "Balanced".into(),
            under_mb: false,
            max_kb: 0,
            resize_percent: Some(50),
            jpg: 80,
            webp: 78,
            png: 2,
            pdf: 72,
            output: "Original".into(),
            preserve: false,
            folder: String::new(),
            replace: false,
            language: "en".into(),
        };
        for ext in ["jpg", "png", "webp"] {
            let source = work.path().join(format!("odd.{ext}"));
            image::RgbImage::new(641, 359).save(&source).unwrap();
            assert_eq!(image_dimensions(&source, &engines).unwrap(), (641, 359));
            let result = images(&source, work.path(), &engines, &options, &|_| {}).unwrap();
            assert_eq!(image_dimensions(&result, &engines).unwrap(), (321, 180));
        }
    }
}
