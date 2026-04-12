use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaKind {
    Image,
    Video,
}

#[derive(Debug, Clone, Copy)]
pub enum VideoOutputProfile {
    Mp4Compatible,
    LinuxWebm,
    MovFast,
    MovHighQuality,
}

impl VideoOutputProfile {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("linux_webm") => Self::LinuxWebm,
            Some("mov_fast") => Self::MovFast,
            Some("mov_high_quality") => Self::MovHighQuality,
            Some("auto") => Self::from_setting(Some(&probe_system_codecs().recommended_profile)),
            Some("mp4_compatible") | None | Some(_) => Self::Mp4Compatible,
        }
    }

    pub fn output_extension(self) -> &'static str {
        match self {
            Self::Mp4Compatible => "mp4",
            Self::LinuxWebm => "webm",
            Self::MovFast | Self::MovHighQuality => "mov",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ImageOutputFormat {
    Jpg,
    Webp,
    Png,
}

impl ImageOutputFormat {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("webp") => Self::Webp,
            Some("png") => Self::Png,
            Some("jpg") | None | Some(_) => Self::Jpg,
        }
    }

    pub fn output_extension(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Webp => "webp",
            Self::Png => "png",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ImageQuality {
    Full,
    Balanced,
    Fast,
}

impl ImageQuality {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("balanced") => Self::Balanced,
            Some("fast") => Self::Fast,
            Some("full") | None | Some(_) => Self::Full,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MediaEncodingOptions {
    pub video_profile: VideoOutputProfile,
    pub image_format: ImageOutputFormat,
    pub image_quality: ImageQuality,
}

#[derive(Debug)]
pub enum MediaError {
    Io(std::io::Error),
    Join(tokio::task::JoinError),
    UnsupportedMediaType(PathBuf),
    MissingOverlay(PathBuf),
    InvalidMetadata(String),
    FfmpegFailed { status: Option<i32>, stderr: String },
}

impl Display for MediaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "media processing I/O failed: {error}"),
            Self::Join(error) => write!(f, "media processing thread failed: {error}"),
            Self::UnsupportedMediaType(path) => {
                write!(
                    f,
                    "unsupported media type for '{}': expected image (jpg/jpeg/png/webp) or video (mp4/mov/m4v/webm)",
                    path.display()
                )
            }
            Self::MissingOverlay(path) => {
                write!(f, "overlay file does not exist at '{}'", path.display())
            }
            Self::InvalidMetadata(reason) => write!(f, "invalid media metadata: {reason}"),
            Self::FfmpegFailed { status, stderr } => {
                write!(
                    f,
                    "ffmpeg exited with status {:?}: {}",
                    status,
                    stderr.trim()
                )
            }
        }
    }
}

impl std::error::Error for MediaError {}

impl From<std::io::Error> for MediaError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<tokio::task::JoinError> for MediaError {
    fn from(value: tokio::task::JoinError) -> Self {
        Self::Join(value)
    }
}

pub async fn merge_media_with_optional_overlay(
    base_media_path: &Path,
    overlay_path: Option<&Path>,
    output_path: &Path,
    encoding_options: MediaEncodingOptions,
) -> Result<(), MediaError> {
    let base_media_path = base_media_path.to_path_buf();
    let overlay_path = overlay_path.map(Path::to_path_buf);
    let output_path = output_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match overlay_path {
            Some(overlay_path) => {
                if !overlay_path.exists() {
                    return Err(MediaError::MissingOverlay(overlay_path));
                }

                let media_kind = media_kind_from_path(&base_media_path)
                    .ok_or_else(|| MediaError::UnsupportedMediaType(base_media_path.clone()))?;

                let ffmpeg_output_path =
                    temporary_output_path_with_suffix(&output_path, "overlay.tmp")?;

                let args = build_ffmpeg_overlay_args(
                    &base_media_path,
                    &overlay_path,
                    &ffmpeg_output_path,
                    media_kind,
                    encoding_options,
                );

                run_ffmpeg(args)?;
                std::fs::rename(&ffmpeg_output_path, &output_path)?;

                Ok(())
            }
            None => {
                let media_kind = media_kind_from_path(&base_media_path)
                    .ok_or_else(|| MediaError::UnsupportedMediaType(base_media_path.clone()))?;

                match media_kind {
                    MediaKind::Image => {
                        let ffmpeg_output_path =
                            temporary_output_path_with_suffix(&output_path, "image.tmp")?;
                        let args = build_ffmpeg_image_transcode_args(
                            &base_media_path,
                            &ffmpeg_output_path,
                            encoding_options,
                        );
                        run_ffmpeg(args)?;
                        std::fs::rename(&ffmpeg_output_path, &output_path)?;
                        Ok(())
                    }
                    MediaKind::Video => {
                        let ffmpeg_output_path =
                            temporary_output_path_with_suffix(&output_path, "normalize.tmp")?;

                        let args = build_ffmpeg_video_normalize_args(
                            &base_media_path,
                            &ffmpeg_output_path,
                            encoding_options.video_profile,
                        );
                        run_ffmpeg(args)?;
                        std::fs::rename(&ffmpeg_output_path, &output_path)?;

                        Ok(())
                    }
                }
            }
        }
    })
    .await??;

    Ok(())
}

pub async fn write_metadata_with_ffmpeg(
    media_path: &Path,
    date_taken: &str,
    location: Option<&str>,
) -> Result<(), MediaError> {
    let media_path = media_path.to_path_buf();
    let date_taken = normalize_datetime_for_ffmpeg(date_taken)?;
    let coordinates = location.and_then(parse_coordinates);

    tokio::task::spawn_blocking(move || {
        let media_kind = media_kind_from_path(&media_path)
            .ok_or_else(|| MediaError::UnsupportedMediaType(media_path.clone()))?;

        let temp_output_path = temporary_output_path(&media_path)?;
        let args = build_ffmpeg_metadata_args(
            &media_path,
            &temp_output_path,
            &date_taken,
            coordinates,
            media_kind,
        );

        run_ffmpeg(args)?;
        std::fs::rename(&temp_output_path, &media_path)?;
        Ok::<(), MediaError>(())
    })
    .await??;

    Ok(())
}

pub async fn cleanup_intermediate_files(
    raw_media_path: &Path,
    overlay_path: Option<&Path>,
    final_media_path: &Path,
) -> Result<(), MediaError> {
    let raw_media_path = raw_media_path.to_path_buf();
    let overlay_path = overlay_path.map(Path::to_path_buf);
    let final_media_path = final_media_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        if raw_media_path != final_media_path {
            remove_file_if_exists(&raw_media_path)?;
        }

        if let Some(overlay_path) = overlay_path {
            if overlay_path != final_media_path {
                remove_file_if_exists(&overlay_path)?;
            }
        }

        Ok::<(), MediaError>(())
    })
    .await??;

    Ok(())
}

fn run_ffmpeg(args: Vec<String>) -> Result<(), MediaError> {
    eprintln!("[ffmpeg] cmd: ffmpeg {}", args.join(" "));

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .map_err(MediaError::Io)?;

    let stderr_text = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        // Log warnings from stderr even on success — encoder warnings about
        // color space mismatches or deprecated options surface here.
        let warnings: Vec<&str> = stderr_text
            .lines()
            .filter(|line| {
                let lower = line.to_ascii_lowercase();
                lower.contains("warning") || lower.contains("discarding")
            })
            .collect();

        if !warnings.is_empty() {
            eprintln!("[ffmpeg] completed with {} warning(s):", warnings.len());
            for warning in warnings {
                eprintln!("[ffmpeg]   {}", warning.trim());
            }
        }

        return Ok(());
    }

    let output_path = args.last().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "[ffmpeg] FAILED status={:?} output='{}'\n[ffmpeg] stderr:\n{}",
        output.status.code(),
        output_path,
        stderr_text.trim()
    );

    Err(MediaError::FfmpegFailed {
        status: output.status.code(),
        stderr: stderr_text.to_string(),
    })
}

fn media_kind_from_path(path: &Path) -> Option<MediaKind> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();

    match extension.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => Some(MediaKind::Image),
        "mp4" | "mov" | "m4v" | "webm" => Some(MediaKind::Video),
        _ => None,
    }
}

fn build_ffmpeg_overlay_args(
    base_media_path: &Path,
    overlay_path: &Path,
    output_path: &Path,
    media_kind: MediaKind,
    encoding_options: MediaEncodingOptions,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        base_media_path.to_string_lossy().to_string(),
        "-i".to_string(),
        overlay_path.to_string_lossy().to_string(),
    ];

    match media_kind {
        MediaKind::Image => {
            args.push("-filter_complex".to_string());
            args.push("[0:v][1:v]overlay=0:0:format=auto".to_string());
            args.push("-frames:v".to_string());
            args.push("1".to_string());
            append_image_encoding_args(
                &mut args,
                encoding_options.image_format,
                encoding_options.image_quality,
            );
        }
        MediaKind::Video => {
            args.push("-filter_complex".to_string());
            args.push(
                "[1:v][0:v]scale2ref[ov][base];[base][ov]overlay=0:0:format=auto,format=yuv420p[vout]"
                    .to_string(),
            );
            args.push("-map".to_string());
            args.push("[vout]".to_string());
            args.push("-map".to_string());
            args.push("0:a?".to_string());
            append_video_encoding_args(&mut args, encoding_options.video_profile);
        }
    }

    args.push(output_path.to_string_lossy().to_string());
    args
}

fn build_ffmpeg_video_normalize_args(
    base_media_path: &Path,
    output_path: &Path,
    video_profile: VideoOutputProfile,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        base_media_path.to_string_lossy().to_string(),
        "-map".to_string(),
        "0:v".to_string(),
        "-map".to_string(),
        "0:a?".to_string(),
        "-dn".to_string(),
    ];

    append_video_encoding_args(&mut args, video_profile);
    args.push(output_path.to_string_lossy().to_string());
    args
}

fn build_ffmpeg_image_transcode_args(
    base_media_path: &Path,
    output_path: &Path,
    encoding_options: MediaEncodingOptions,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        base_media_path.to_string_lossy().to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
    ];

    append_image_encoding_args(
        &mut args,
        encoding_options.image_format,
        encoding_options.image_quality,
    );
    args.push(output_path.to_string_lossy().to_string());
    args
}

fn append_video_encoding_args(args: &mut Vec<String>, profile: VideoOutputProfile) {
    match profile {
        VideoOutputProfile::Mp4Compatible => {
            args.push("-c:v".to_string());
            args.push("libx264".to_string());
            args.push("-preset".to_string());
            args.push("veryfast".to_string());
            args.push("-crf".to_string());
            args.push("18".to_string());
            args.push("-profile:v".to_string());
            args.push("high".to_string());
            args.push("-pix_fmt".to_string());
            args.push("yuv420p".to_string());
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("128k".to_string());
            args.push("-movflags".to_string());
            args.push("+faststart".to_string());
        }
        VideoOutputProfile::LinuxWebm => {
            args.push("-c:v".to_string());
            args.push("libvpx-vp9".to_string());
            args.push("-pix_fmt".to_string());
            args.push("yuv420p".to_string());
            args.push("-b:v".to_string());
            args.push("0".to_string());
            args.push("-crf".to_string());
            args.push("20".to_string());
            args.push("-g".to_string());
            args.push("240".to_string());
            args.push("-tile-columns".to_string());
            args.push("2".to_string());
            args.push("-tile-rows".to_string());
            args.push("1".to_string());
            args.push("-row-mt".to_string());
            args.push("1".to_string());
            args.push("-deadline".to_string());
            args.push("good".to_string());
            args.push("-cpu-used".to_string());
            args.push("3".to_string());
            args.push("-error-resilient".to_string());
            args.push("1".to_string());
            args.push("-c:a".to_string());
            args.push("libopus".to_string());
            args.push("-b:a".to_string());
            args.push("128k".to_string());
            args.push("-f".to_string());
            args.push("webm".to_string());
        }
        VideoOutputProfile::MovFast => {
            args.push("-c:v".to_string());
            args.push("libx264".to_string());
            args.push("-preset".to_string());
            args.push("ultrafast".to_string());
            args.push("-crf".to_string());
            args.push("23".to_string());
            args.push("-profile:v".to_string());
            args.push("main".to_string());
            args.push("-pix_fmt".to_string());
            args.push("yuv420p".to_string());
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("128k".to_string());
            args.push("-movflags".to_string());
            args.push("+faststart".to_string());
        }
        VideoOutputProfile::MovHighQuality => {
            args.push("-c:v".to_string());
            args.push("libx264".to_string());
            args.push("-preset".to_string());
            args.push("slow".to_string());
            args.push("-crf".to_string());
            args.push("16".to_string());
            args.push("-profile:v".to_string());
            args.push("high".to_string());
            args.push("-pix_fmt".to_string());
            args.push("yuv420p".to_string());
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("192k".to_string());
            args.push("-movflags".to_string());
            args.push("+faststart".to_string());
        }
    }

    // Explicit color space metadata for consistent decoding across players.
    // Without these flags, GStreamer (used by WebKitGTK on Linux) may
    // misinterpret the color space, causing wrong colors and visual artifacts.
    args.push("-colorspace".to_string());
    args.push("bt709".to_string());
    args.push("-color_primaries".to_string());
    args.push("bt709".to_string());
    args.push("-color_trc".to_string());
    args.push("bt709".to_string());
    args.push("-color_range".to_string());
    args.push("tv".to_string());

    args.push("-max_muxing_queue_size".to_string());
    args.push("1024".to_string());
}

fn append_image_encoding_args(
    args: &mut Vec<String>,
    image_format: ImageOutputFormat,
    image_quality: ImageQuality,
) {
    match image_format {
        ImageOutputFormat::Jpg => {
            args.push("-c:v".to_string());
            args.push("mjpeg".to_string());
            args.push("-q:v".to_string());
            args.push(
                match image_quality {
                    ImageQuality::Full => "2",
                    ImageQuality::Balanced => "5",
                    ImageQuality::Fast => "8",
                }
                .to_string(),
            );
        }
        ImageOutputFormat::Webp => {
            args.push("-c:v".to_string());
            args.push("libwebp".to_string());
            args.push("-quality".to_string());
            args.push(
                match image_quality {
                    ImageQuality::Full => "100",
                    ImageQuality::Balanced => "86",
                    ImageQuality::Fast => "72",
                }
                .to_string(),
            );
            args.push("-compression_level".to_string());
            args.push(
                match image_quality {
                    ImageQuality::Full => "6",
                    ImageQuality::Balanced => "4",
                    ImageQuality::Fast => "2",
                }
                .to_string(),
            );
        }
        ImageOutputFormat::Png => {
            args.push("-c:v".to_string());
            args.push("png".to_string());
            args.push("-compression_level".to_string());
            args.push(
                match image_quality {
                    ImageQuality::Full => "9",
                    ImageQuality::Balanced => "6",
                    ImageQuality::Fast => "2",
                }
                .to_string(),
            );
        }
    }
}

fn build_ffmpeg_metadata_args(
    media_path: &Path,
    output_path: &Path,
    date_taken: &str,
    coordinates: Option<(f64, f64)>,
    media_kind: MediaKind,
) -> Vec<String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        media_path.to_string_lossy().to_string(),
        "-metadata".to_string(),
        format!("DateTimeOriginal={date_taken}"),
        "-metadata".to_string(),
        format!("DateTimeDigitized={date_taken}"),
        "-metadata".to_string(),
        format!("creation_time={date_taken}"),
    ];

    if let Some((latitude, longitude)) = coordinates {
        args.push("-metadata".to_string());
        args.push(format!("GPSLatitude={latitude}"));
        args.push("-metadata".to_string());
        args.push(format!("GPSLongitude={longitude}"));
        args.push("-metadata".to_string());
        args.push(format!(
            "location={}{}{}{}",
            if latitude >= 0.0 { "+" } else { "" },
            latitude,
            if longitude >= 0.0 { "+" } else { "" },
            longitude
        ));
    }

    match media_kind {
        MediaKind::Image => {
            args.push("-frames:v".to_string());
            args.push("1".to_string());
        }
        MediaKind::Video => {
            args.push("-map".to_string());
            args.push("0:v".to_string());
            args.push("-map".to_string());
            args.push("0:a?".to_string());
            args.push("-dn".to_string());
            args.push("-c".to_string());
            args.push("copy".to_string());
        }
    }

    args.push(output_path.to_string_lossy().to_string());
    args
}

fn normalize_datetime_for_ffmpeg(value: &str) -> Result<String, MediaError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MediaError::InvalidMetadata("date is empty".to_string()));
    }

    if trimmed.len() >= 19 && trimmed.as_bytes().get(10) == Some(&b'T') {
        let date = &trimmed[..10];
        let time = &trimmed[11..19];
        return Ok(format!("{} {}", date.replace('-', ":"), time));
    }

    if trimmed.len() >= 19
        && trimmed.as_bytes().get(4) == Some(&b'-')
        && trimmed.as_bytes().get(7) == Some(&b'-')
        && trimmed.as_bytes().get(10) == Some(&b' ')
    {
        let date = &trimmed[..10];
        let time = &trimmed[11..19];
        return Ok(format!("{} {}", date.replace('-', ":"), time));
    }

    Ok(trimmed.to_string())
}

fn parse_coordinates(value: &str) -> Option<(f64, f64)> {
    let mut parts = value.split(',').map(str::trim);
    let latitude = parts.next()?.parse::<f64>().ok()?;
    let longitude = parts.next()?.parse::<f64>().ok()?;
    Some((latitude, longitude))
}

fn temporary_output_path(media_path: &Path) -> Result<PathBuf, MediaError> {
    temporary_output_path_with_suffix(media_path, "metadata.tmp")
}

fn temporary_output_path_with_suffix(
    media_path: &Path,
    suffix: &str,
) -> Result<PathBuf, MediaError> {
    let stem = media_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            MediaError::InvalidMetadata("media path does not contain a valid file stem".to_string())
        })?;

    let extension = media_path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            MediaError::InvalidMetadata("media path does not contain a valid extension".to_string())
        })?;

    Ok(media_path.with_file_name(format!("{stem}.{suffix}.{extension}")))
}

fn remove_file_if_exists(path: &Path) -> Result<(), MediaError> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}

/// Probes the system for GStreamer codec availability.
///
/// Returns a [`SystemCodecInfo`] describing which video profiles are likely to
/// play back correctly in WebKitGTK's GStreamer pipeline.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemCodecInfo {
    pub has_h264_decoder: bool,
    pub has_vp9_decoder: bool,
    pub has_opus_decoder: bool,
    pub has_aac_decoder: bool,
    pub recommended_profile: String,
}

pub fn probe_system_codecs() -> SystemCodecInfo {
    let has_h264 = gst_element_exists("avdec_h264")
        || gst_element_exists("openh264dec")
        || gst_element_exists("vaapih264dec")
        || gst_element_exists("vah264dec");

    let has_vp9 = gst_element_exists("vp9dec")
        || gst_element_exists("vp9alphadecodebin")
        || gst_element_exists("vaapivp9dec")
        || gst_element_exists("vavp9dec");

    let has_opus = gst_element_exists("opusdec");
    let has_aac = gst_element_exists("avdec_aac") || gst_element_exists("faad");

    let recommended = if has_h264 && has_aac {
        "mp4_compatible"
    } else if has_vp9 && has_opus {
        "linux_webm"
    } else {
        "mp4_compatible"
    };

    SystemCodecInfo {
        has_h264_decoder: has_h264,
        has_vp9_decoder: has_vp9,
        has_opus_decoder: has_opus,
        has_aac_decoder: has_aac,
        recommended_profile: recommended.to_string(),
    }
}

fn gst_element_exists(element_name: &str) -> bool {
    Command::new("gst-inspect-1.0")
        .arg(element_name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Runs `ffprobe` on the output to verify it has at least one video stream
/// and that its pixel format and codec match expectations.
pub async fn verify_video_integrity(
    video_path: &Path,
    expected_profile: VideoOutputProfile,
) -> Result<bool, MediaError> {
    let video_path = video_path.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,pix_fmt,width,height",
                "-of",
                "csv=p=0",
            ])
            .arg(&video_path)
            .output()
            .map_err(MediaError::Io)?;

        if !output.status.success() {
            eprintln!(
                "[media-verify] ffprobe failed for '{}': {}",
                video_path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.trim();

        if line.is_empty() {
            eprintln!(
                "[media-verify] no video stream found in '{}'",
                video_path.display()
            );
            return Ok(false);
        }

        // ffprobe csv output: codec_name,pix_fmt,width,height
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 2 {
            eprintln!(
                "[media-verify] unexpected ffprobe output for '{}': {line}",
                video_path.display()
            );
            return Ok(false);
        }

        let codec = parts[0].trim();
        let pix_fmt = parts[1].trim();

        let expected_codec = match expected_profile {
            VideoOutputProfile::Mp4Compatible
            | VideoOutputProfile::MovFast
            | VideoOutputProfile::MovHighQuality => "h264",
            VideoOutputProfile::LinuxWebm => "vp9",
        };

        if codec != expected_codec {
            eprintln!(
                "[media-verify] codec mismatch for '{}': expected={expected_codec} actual={codec}",
                video_path.display()
            );
            return Ok(false);
        }

        if pix_fmt != "yuv420p" {
            eprintln!(
                "[media-verify] pix_fmt mismatch for '{}': expected=yuv420p actual={pix_fmt}",
                video_path.display()
            );
            return Ok(false);
        }

        Ok(true)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use tempfile::tempdir;

    use super::{
        build_ffmpeg_metadata_args, build_ffmpeg_overlay_args, cleanup_intermediate_files,
        media_kind_from_path, normalize_datetime_for_ffmpeg, parse_coordinates, ImageOutputFormat,
        ImageQuality, MediaEncodingOptions, MediaKind, VideoOutputProfile,
    };

    #[test]
    fn detects_supported_media_kinds() {
        assert_eq!(
            media_kind_from_path(Path::new("memory.jpg")),
            Some(MediaKind::Image)
        );
        assert_eq!(
            media_kind_from_path(Path::new("memory.JPEG")),
            Some(MediaKind::Image)
        );
        assert_eq!(
            media_kind_from_path(Path::new("memory.mp4")),
            Some(MediaKind::Video)
        );
        assert_eq!(
            media_kind_from_path(Path::new("memory.mov")),
            Some(MediaKind::Video)
        );
        assert_eq!(
            media_kind_from_path(Path::new("memory.webm")),
            Some(MediaKind::Video)
        );
    }

    #[test]
    fn builds_overlay_arguments_for_images() {
        let args = build_ffmpeg_overlay_args(
            Path::new("base.jpg"),
            Path::new("overlay.png"),
            Path::new("output.jpg"),
            MediaKind::Image,
            MediaEncodingOptions {
                video_profile: VideoOutputProfile::Mp4Compatible,
                image_format: ImageOutputFormat::Jpg,
                image_quality: ImageQuality::Full,
            },
        );

        assert!(args.contains(&"-frames:v".to_string()));
        assert!(!args.contains(&"libx264".to_string()));
    }

    #[test]
    fn builds_overlay_arguments_for_videos() {
        let args = build_ffmpeg_overlay_args(
            Path::new("base.mp4"),
            Path::new("overlay.png"),
            Path::new("output.mp4"),
            MediaKind::Video,
            MediaEncodingOptions {
                video_profile: VideoOutputProfile::Mp4Compatible,
                image_format: ImageOutputFormat::Jpg,
                image_quality: ImageQuality::Full,
            },
        );

        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"0:a?".to_string()));
        assert!(!args.contains(&"-frames:v".to_string()));
    }

    #[test]
    fn normalizes_rfc3339_datetime_for_ffmpeg() {
        let formatted = normalize_datetime_for_ffmpeg("2024-03-01T12:13:14Z")
            .expect("datetime should be normalized");
        assert_eq!(formatted, "2024:03:01 12:13:14");
    }

    #[test]
    fn parses_coordinates_from_location_string() {
        let coordinates = parse_coordinates("48.8566,2.3522").expect("coordinates should parse");
        assert_eq!(coordinates.0, 48.8566);
        assert_eq!(coordinates.1, 2.3522);
    }

    #[test]
    fn builds_metadata_arguments_with_gps_tags() {
        let args = build_ffmpeg_metadata_args(
            Path::new("output.jpg"),
            Path::new("output.jpg.metadata.tmp"),
            "2024:03:01 12:13:14",
            Some((12.34, -56.78)),
            MediaKind::Image,
        );

        assert!(args
            .iter()
            .any(|arg| arg.contains("DateTimeOriginal=2024:03:01 12:13:14")));
        assert!(args.iter().any(|arg| arg.contains("GPSLatitude=12.34")));
        assert!(args.iter().any(|arg| arg.contains("GPSLongitude=-56.78")));
    }

    #[tokio::test]
    async fn cleanup_deletes_raw_and_overlay_keeps_final() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let raw_path = temp_dir.path().join("raw.jpg");
        let overlay_path = temp_dir.path().join("overlay.png");
        let final_path = temp_dir.path().join("final.jpg");

        std::fs::write(&raw_path, b"raw").expect("raw file should be created");
        std::fs::write(&overlay_path, b"overlay").expect("overlay file should be created");
        std::fs::write(&final_path, b"final").expect("final file should be created");

        cleanup_intermediate_files(&raw_path, Some(&overlay_path), &final_path)
            .await
            .expect("cleanup should succeed");

        assert!(!raw_path.exists());
        assert!(!overlay_path.exists());
        assert!(final_path.exists());
    }

    #[tokio::test]
    async fn cleanup_does_not_delete_final_when_paths_match() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let final_path = temp_dir.path().join("final.jpg");

        std::fs::write(&final_path, b"final").expect("final file should be created");

        cleanup_intermediate_files(&final_path, Some(&final_path), &final_path)
            .await
            .expect("cleanup should succeed when paths match");

        assert!(final_path.exists());
    }
}
