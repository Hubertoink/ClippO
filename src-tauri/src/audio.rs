use crate::engine::{AppResult, Explain, Options};
use hound::{SampleFormat, WavReader};
use mp3lame_encoder::{
    max_required_buffer_size, Bitrate, Builder, FlushGap, InterleavedPcm, MonoPcm,
};
use std::{
    fs::File,
    io::{BufWriter, Seek, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn bitrate(options: &Options, frames: u32, rate: u32) -> Bitrate {
    use Bitrate::*;
    let preferred = match options.mode.as_str() {
        "Smart" => 256,
        "Maximum" => 96,
        "Custom" => options.mp3_kbps.unwrap_or(192),
        _ => 192,
    };
    // Budget for two extra MP3 frames and the seek header.
    let seconds = f64::from(frames) / f64::from(rate);
    let budget_kbps = options.target().saturating_sub(2048) as f64 * 0.008 / seconds;
    [
        Kbps320, Kbps256, Kbps224, Kbps192, Kbps160, Kbps128, Kbps112, Kbps96, Kbps80, Kbps64,
        Kbps48, Kbps40, Kbps32, Kbps24, Kbps16,
    ]
    .into_iter()
    .find(|b| (*b as u16) <= preferred && f64::from(*b as u16) <= budget_kbps)
    .unwrap_or(Kbps8)
}

pub fn to_mp3(
    input: &Path,
    work: &Path,
    options: &Options,
    progress: &dyn Fn(u64),
) -> AppResult<PathBuf> {
    let german = options.language == "de";
    let invalid = if german {
        "Bitte eine gültige Mono- oder Stereo-WAV mit PCM- oder Float-Samples wählen."
    } else {
        "Choose a valid mono or stereo PCM or floating-point WAV file."
    };
    let mut wav = WavReader::open(input).map_err(|_| invalid)?;
    let spec = wav.spec();
    if !(1..=2).contains(&spec.channels)
        || !(8000..=384000).contains(&spec.sample_rate)
        || wav.duration() == 0
        || !(1..=32).contains(&spec.bits_per_sample)
    {
        return Err(invalid.into());
    }
    let mut builder = Builder::new().ok_or("Could not initialize the MP3 encoder.")?;
    builder.set_num_channels(spec.channels as u8).explain()?;
    builder.set_sample_rate(spec.sample_rate).explain()?;
    builder
        .set_brate(bitrate(options, wav.duration(), spec.sample_rate))
        .explain()?;
    builder.set_to_write_vbr_tag(true).explain()?;
    let mut encoder = builder.build().explain()?;
    let mut samples: Box<dyn Iterator<Item = hound::Result<f32>> + '_> = match spec.sample_format {
        SampleFormat::Float => Box::new(wav.samples::<f32>()),
        SampleFormat::Int => {
            let scale = 2_f32.powi(i32::from(spec.bits_per_sample) - 1);
            Box::new(
                wav.samples::<i32>()
                    .map(move |sample| sample.map(|n| n as f32 / scale)),
            )
        }
    };
    let path = work.join("converted.mp3");
    let mut output = BufWriter::new(File::create(&path).explain()?);
    const FRAMES: usize = 8192;
    let chunk = FRAMES * usize::from(spec.channels);
    let mut pcm = Vec::with_capacity(chunk);
    let mut encoded = Vec::with_capacity(max_required_buffer_size(FRAMES));
    let mut bytes = 0;
    let mut last_progress = Instant::now();
    loop {
        pcm.clear();
        for sample in samples.by_ref().take(chunk) {
            let sample = sample.map_err(|_| invalid)?;
            if !sample.is_finite() {
                return Err(invalid.into());
            }
            pcm.push(sample.clamp(-1., 1.));
        }
        if pcm.is_empty() {
            break;
        }
        encoded.clear();
        if spec.channels == 1 {
            encoder.encode_to_vec(MonoPcm(&pcm), &mut encoded)
        } else {
            encoder.encode_to_vec(InterleavedPcm(&pcm), &mut encoded)
        }
        .explain()?;
        output.write_all(&encoded).explain()?;
        bytes += encoded.len() as u64;
        if last_progress.elapsed() >= Duration::from_millis(150) {
            progress(bytes);
            last_progress = Instant::now();
        }
    }
    encoded.clear();
    encoder.flush_to_vec::<FlushGap>(&mut encoded).explain()?;
    output.write_all(&encoded).explain()?;
    bytes += encoded.len() as u64;
    // Replace LAME's reserved first frame with the final duration and padding.
    if encoder.lame_tag_size() > 0 {
        let mut tag = Vec::with_capacity(encoder.lame_tag_size());
        encoder
            .lame_tag_encode_to_vec(&mut tag)
            .ok_or("Could not finalize the MP3 header.")?;
        output.rewind().explain()?;
        output.write_all(&tag).explain()?;
    }
    output.flush().explain()?;
    progress(bytes);
    Ok(path)
}
