use clap::{Parser, Subcommand};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageReader, ImageResult};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

/// kpic compression core
#[derive(Parser)]
#[command(name = "imgsqueeze", about = "Compress images from the command line")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// compress one or more images
    Compress {
        /// input image files
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// output quality 1-100 (lossy formats only)
        #[arg(long, default_value_t = 80)]
        quality: u8,

        /// output format: jpg | png | webp. omit to keep input format.
        #[arg(long)]
        format: Option<String>,

        /// lossless optimization (png/webp; ignored for jpeg)
        #[arg(long)]
        lossless: bool,
    },
}

#[derive(Debug)]
struct CompressResult {
    input: PathBuf,
    output: PathBuf,
    input_size: u64,
    output_size: u64,
}

#[derive(Debug)]
struct Job {
    input: PathBuf,
    output: PathBuf,
    format: String,
}

fn output_ext(format: &str) -> &'static str {
    match format {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        _ => "webp",
    }
}

/// detect image format from file signature (not extension).
fn detect_format(path: &Path) -> Option<&'static str> {
    match ImageReader::open(path).ok()?.with_guessed_format().ok()?.format() {
        Some(image::ImageFormat::Jpeg) => Some("jpg"),
        Some(image::ImageFormat::Png) => Some("png"),
        Some(image::ImageFormat::WebP) => Some("webp"),
        _ => None,
    }
}

/// resolve output format + unique output name per input, serially.
/// `override_format` converts all files; `None` keeps each input format.
fn resolve_jobs(
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

fn encode_image(
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

fn compress_image(
    input: &Path,
    output: &Path,
    format: &str,
    quality: u8,
    lossless: bool,
) -> ImageResult<u64> {
    let img = ImageReader::open(input)?.with_guessed_format()?.decode()?;
    let bytes = encode_image(&img, format, quality, lossless)?;
    fs::write(output, bytes)?;
    Ok(fs::metadata(output)?.len())
}

fn format_bytes(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}MB", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}KB", n as f64 / 1_000.0)
    } else {
        format!("{n}B")
    }
}

fn main() {
    let cli = Cli::parse();

    let Commands::Compress {
        files,
        quality,
        format,
        lossless,
    } = cli.command;

    let format = format.map(|f| f.to_lowercase());
    if let Some(f) = &format {
        if !matches!(f.as_str(), "jpg" | "jpeg" | "png" | "webp") {
            eprintln!("unsupported format: {f} (use jpg, png, webp)");
            std::process::exit(1);
        }
    }

    let jobs = resolve_jobs(&files, format.as_deref());

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("[{pos}/{len}] {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    let results: Vec<Result<CompressResult, String>> = jobs
        .into_par_iter()
        .map(|job| {
            let job = job?;
            let input_size = fs::metadata(&job.input)
                .map_err(|e| format!("{}: {}", job.input.display(), e))?
                .len();
            pb.inc(1);
            let output_size = compress_image(
                &job.input,
                &job.output,
                &job.format,
                quality,
                lossless,
            )
            .map_err(|e| format!("{}: {}", job.input.display(), e))?;
            Ok(CompressResult {
                input: job.input,
                output: job.output,
                input_size,
                output_size,
            })
        })
        .collect();

    pb.finish();

    let mut ok = 0;
    for r in results {
        match r {
            Ok(r) => {
                ok += 1;
                let saved = format_bytes(r.input_size.saturating_sub(r.output_size));
                let sign = if r.output_size < r.input_size { "-" } else { "+" };
                println!(
                    "{} {} -> {} ({} -> {}, {} saved)",
                    r.input.display(),
                    sign,
                    r.output.display(),
                    format_bytes(r.input_size),
                    format_bytes(r.output_size),
                    saved,
                );
            }
            Err(e) => eprintln!("error: {e}"),
        }
    }
    println!("done: {ok}/{} compressed", files.len());
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
}
