use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageReader, ImageResult};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

mod kpic_png;

#[derive(Debug)]
pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
    pub format: String,
}

pub fn output_ext(format: &str) -> &'static str {
    match format {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        _ => "webp",
    }
}

/// detect image format from file signature (not extension).
pub fn detect_format(path: &Path) -> Option<&'static str> {
    match ImageReader::open(path).ok()?.with_guessed_format().ok()?.format() {
        Some(image::ImageFormat::Jpeg) => Some("jpg"),
        Some(image::ImageFormat::Png) => Some("png"),
        Some(image::ImageFormat::WebP) => Some("webp"),
        _ => None,
    }
}

/// resolve output format + unique output name per input, serially.
/// `override_format` converts all files; `None` keeps each input format.
pub fn resolve_jobs(
    inputs: &[PathBuf],
    override_format: Option<&str>,
) -> Vec<Result<Job, String>> {
    let mut used: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    inputs
        .iter()
        .map(|input| {
            let format = match override_format {
                Some(f) => f.to_string(),
                None => detect_format(input)
                    .ok_or_else(|| format!("{}: unsupported input format", input.display()))?
                    .to_string(),
            };
            let stem = input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            let dir = input.parent().unwrap_or_else(|| Path::new("."));
            let mut candidate = dir.join(format!("{stem}-compressed.{}", output_ext(&format)));
            let mut n = 2;
            while used.contains(&candidate) || candidate.exists() {
                candidate = dir.join(format!("{stem}-compressed-{n}.{}", output_ext(&format)));
                n += 1;
            }
            used.insert(candidate.clone());
            Ok(Job {
                input: input.clone(),
                output: candidate,
                format,
            })
        })
        .collect()
}

pub fn encode_image(
    img: &DynamicImage,
    format: &str,
    quality: u8,
    lossless: bool,
) -> ImageResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());

    match format {
        "jpg" | "jpeg" => {
            // jpeg has no alpha, flatten and force lossy encode.
            let rgb = img.to_rgb8();
            JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100))
                .encode_image(&DynamicImage::ImageRgb8(rgb))?;
        }
        "png" => {
            let rgba = img.to_rgba8();
            PngEncoder::new(&mut buf).write_image(
                rgba.as_raw().as_slice(),
                rgba.width(),
                rgba.height(),
                ExtendedColorType::Rgba8,
            )?;
        }
        "png-lossy" => {
            // lossy palette quantization, keep slider meaning against the carrier
            // format's intrinsic ceiling.
            let rgba = img.to_rgba8();
            let bytes = kpic_png::quantize(rgba.as_raw(), rgba.width(), rgba.height(), quality)?;
            buf.write_all(&bytes)?;
        }
        "webp" => {
            let rgba = img.to_rgba8();
            let encoder = webp::Encoder::from_rgba(
                rgba.as_raw().as_slice(),
                rgba.width(),
                rgba.height(),
            );
            let image = if lossless {
                encoder.encode_lossless()
            } else {
                encoder.encode(quality as f32)
            };
            buf.write_all(image.as_ref())?;
        }
        _ => {
            return Err(image::ImageError::Decoding(
                image::error::DecodingError::new(
                    image::error::ImageFormatHint::Unknown,
                    format!("unsupported output format: {format}"),
                ),
            ))
        }
    }

    Ok(buf.into_inner())
}

pub fn compress_image(
    input: &Path,
    output: &Path,
    format: &str,
    quality: u8,
    lossless: bool,
) -> ImageResult<u64> {
    // png in, png out, lossless: pixels and metadata stay exact, use oxipng
    // lossless optimizer on the original bytes, never grow the file.
    // lossy png goes through palette quantization instead.
    if format == "png" && lossless {
        let input_bytes = fs::read(input)?;
        let optimized = oxipng::optimize_from_memory(&input_bytes, &oxipng::Options::from_preset(2))
            .map_err(|e| {
                image::ImageError::Decoding(image::error::DecodingError::new(
                    image::error::ImageFormatHint::Unknown,
                    format!("oxipng: {e}"),
                ))
            })?;
        let out_bytes = if optimized.len() < input_bytes.len() {
            optimized
        } else {
            input_bytes
        };
        fs::write(output, out_bytes)?;
        return Ok(fs::metadata(output)?.len());
    }
    let img = ImageReader::open(input)?.with_guessed_format()?.decode()?;
    let eff_format = if format == "png" { "png-lossy" } else { format };
    let bytes = encode_image(&img, eff_format, quality, lossless)?;
    // skip-if-larger: never write an output that grows the input.
    if format == "png"
        && bytes.len() as u64 >= fs::metadata(input)?.len()
    {
        return Err(image::ImageError::Decoding(
            image::error::DecodingError::new(
                image::error::ImageFormatHint::Unknown,
                "skip-if-larger: quantization does not shrink this image",
            ),
        ));
    }
    fs::write(output, bytes)?;
    Ok(fs::metadata(output)?.len())
}

pub struct CompressOptions {
    pub quality: u8,
    pub lossless: bool,
    /// keep input format.
    pub format: Option<String>,
}

#[derive(Debug, Default)]
pub struct FileResult {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub input_size: u64,
    pub output_size: Option<u64>,
    pub error: Option<String>,
}

/// wrap compress_image, error prefixed with input path.
pub fn compress_one(
    input: &Path,
    output: &Path,
    format: &str,
    quality: u8,
    lossless: bool,
) -> Result<u64, String> {
    compress_image(input, output, format, quality, lossless)
        .map_err(|e| format!("{}: {}", input.display(), e))
}

pub struct BatchProgress {
    pub callback: Box<dyn Fn(usize) + Send>,
}

/// compress a batch serially, collision-safe names, cancel-aware.
/// stop flag true between files skips the rest with error entries.
/// progress callback fires after each file with the finished count.
pub fn run_batch(
    inputs: &[PathBuf],
    opts: &CompressOptions,
    cancel: Option<&std::sync::atomic::AtomicBool>,
    progress: Option<&BatchProgress>,
) -> Vec<FileResult> {
    let jobs = resolve_jobs(inputs, opts.format.as_deref());
    let mut out = Vec::with_capacity(jobs.len());
    let mut done = 0usize;
    for (input, job) in inputs.iter().zip(jobs) {
        if cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)) {
            out.push(FileResult {
                input: input.clone(),
                error: Some("cancelled".into()),
                ..Default::default()
            });
        } else {
            match job {
                Err(e) => out.push(FileResult {
                    input: input.clone(),
                    error: Some(e),
                    ..Default::default()
                }),
                Ok(job) => {
                    let input_size = std::fs::metadata(&job.input)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    match compress_one(&job.input, &job.output, &job.format, opts.quality, opts.lossless) {
                        Ok(output_size) => out.push(FileResult {
                            input: job.input,
                            output: Some(job.output),
                            input_size,
                            output_size: Some(output_size),
                            ..Default::default()
                        }),
                        Err(e) => out.push(FileResult {
                            input: job.input,
                            output: Some(job.output),
                            input_size,
                            output_size: None,
                            error: Some(e),
                        }),
                    }
                }
            }
        }
        done += 1;
        if let Some(p) = progress {
            (p.callback)(done);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    /// deterministic 64x48 test image with a gradient + a solid block.
    fn test_image() -> DynamicImage {
        let mut img = RgbaImage::new(64, 48);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([
                x as u8,
                y as u8,
                (x + y) as u8,
                255,
            ]);
        }
        DynamicImage::ImageRgba8(img)
    }

    fn decode(bytes: &[u8], kind: image::ImageFormat) -> DynamicImage {
        image::load_from_memory_with_format(bytes, kind).unwrap()
    }

    #[test]
    fn encode_decode_jpeg_roundtrip() {
        let bytes = encode_image(&test_image(), "jpg", 80, false).unwrap();
        let out = decode(&bytes, image::ImageFormat::Jpeg);
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 48);
    }

    #[test]
    fn encode_decode_png_roundtrip() {
        let bytes = encode_image(&test_image(), "png", 80, false).unwrap();
        let out = decode(&bytes, image::ImageFormat::Png);
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 48);
    }

    #[test]
    fn png_lossless_optimizes_or_copies_never_grows() {
        let dir = std::env::temp_dir();
        let input = dir.join("kpic-lossless-input.png");
        let output = dir.join("kpic-lossless-output.png");
        // sloppy png source: image crate default encode, not optimized
        let raw = encode_image(&test_image(), "png", 100, false).unwrap();
        fs::write(&input, &raw).unwrap();
        let out_size = compress_image(&input, &output, "png", 0, true).unwrap();
        let input_len = fs::metadata(&input).unwrap().len();
        assert!(
            out_size <= input_len,
            "lossless grew the file: {out_size} > {input_len}"
        );
        // pixels bit-exact against source
        let out = decode(&fs::read(&output).unwrap(), image::ImageFormat::Png);
        assert_eq!(out.to_rgba8().as_raw(), test_image().to_rgba8().as_raw());
        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
    }

    #[test]
    fn png_growth_skipped_not_written() {
        // 1x1 solid png cannot be quantized smaller; compress_image must
        // refuse instead of writing a larger file.
        let dir = std::env::temp_dir();
        let input = dir.join("kpic-tiny-input.png");
        let output = dir.join("kpic-tiny-output.png");
        let tiny = image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 150, 100, 255]));
        let tiny_png = {
            let mut buf = Cursor::new(Vec::new());
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(
                    tiny.as_raw(),
                    tiny.width(),
                    tiny.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
            buf.into_inner()
        };
        fs::write(&input, &tiny_png).unwrap();
        let err = compress_image(&input, &output, "png", 50, false).unwrap_err();
        assert!(
            format!("{err}").contains("skip-if-larger"),
            "unexpected error: {err}"
        );
        assert!(!output.exists(), "oversized output must not be written");
        let _ = fs::remove_file(&input);
    }

    #[test]
    fn encode_decode_webp_lossless_roundtrip() {
        let bytes = encode_image(&test_image(), "webp", 80, true).unwrap();
        let out = decode(&bytes, image::ImageFormat::WebP);
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 48);
    }

    #[test]
    fn encode_decode_webp_lossy_roundtrip() {
        let bytes = encode_image(&test_image(), "webp", 80, false).unwrap();
        let out = decode(&bytes, image::ImageFormat::WebP);
        assert_eq!(out.width(), 64);
        assert_eq!(out.height(), 48);
    }

    #[test]
    fn lossless_webp_roundtrip_is_exact() {
        let img = test_image();
        let bytes = encode_image(&img, "webp", 80, true).unwrap();
        let out = decode(&bytes, image::ImageFormat::WebP);
        // lossless webp must reproduce pixels bit-for-bit.
        assert_eq!(out.to_rgba8().clone().into_raw(), img.to_rgba8().into_raw());
    }

    #[test]
    fn lossy_webp_smaller_than_lossless() {
        // lossy encoding must be smaller than lossless.
        let mut img = RgbaImage::new(64, 48);
        let mut seed = 0x12345678u32;
        for (_, _, p) in img.enumerate_pixels_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *p = image::Rgba([seed as u8, (seed >> 8) as u8, (seed >> 16) as u8, 255]);
        }
        let img = DynamicImage::ImageRgba8(img);
        let lossy = encode_image(&img, "webp", 60, false).unwrap();
        let lossless = encode_image(&img, "webp", 60, true).unwrap();
        assert!(lossy.len() < lossless.len());
    }

    #[test]
    fn unsupported_format_errors() {
        let err = encode_image(&test_image(), "gif", 80, false);
        assert!(err.is_err());
    }

    #[test]
    fn resolve_jobs_avoids_collisions() {
        let dir = std::env::temp_dir();
        let a = dir.join("photo.png");
        let b = dir.join("photo.webp");
        // force the ideal name to appear taken.
        std::fs::write(dir.join("photo-compressed.png"), b"x").unwrap();
        let jobs: Vec<Job> = resolve_jobs(&[a, b], Some("png"))
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        assert_ne!(jobs[0].output, jobs[1].output);
        for j in &jobs {
            assert!(j.output.to_str().unwrap().contains("-compressed"));
        }
        std::fs::remove_file(dir.join("photo-compressed.png")).ok();
    }

    #[test]
    fn run_batch_produces_results_in_order() {
        let tmp = std::env::temp_dir().join(format!("kpic-task2-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let img_path = tmp.join("grad.png");
        // generate a real png so format detection succeeds
        let mut png = image::RgbaImage::new(8, 8);
        for (_, _, p) in png.enumerate_pixels_mut() {
            *p = image::Rgba([200, 100, 50, 255]);
        }
        let mut file = std::fs::File::create(&img_path).unwrap();
        image::codecs::png::PngEncoder::new(&mut file)
            .write_image(png.as_raw(), 8, 8, image::ExtendedColorType::Rgba8)
            .unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let opts = CompressOptions { quality: 80, lossless: false, format: None };
        let res = run_batch(&[img_path.clone()], &opts, Some(&cancel), None);
        let r = &res[0];
        assert!(r.output.is_some(), "successful file must have output");
        assert!(r.error.is_none());
        assert!(r.output_size.unwrap() > 0);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn run_batch_skips_remaining_after_cancel() {
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let opts = CompressOptions { quality: 80, lossless: false, format: Some("png".into()) };
        // cancel checked before touching files
        let res = run_batch(&["a.png".into(), "b.png".into()], &opts, Some(&cancel), None);
        assert_eq!(res.len(), 2);
        assert!(res[1].error.is_some(), "second file skipped with cancel flag");
    }

    #[test]
    fn compress_one_reports_path_prefixed_error() {
        let err = compress_one(&Path::new("/nonexistent/x.png"), &Path::new("/nonexistent/x-c.png"), "webp", 80, false);
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("/nonexistent/x.png"));
    }
}
