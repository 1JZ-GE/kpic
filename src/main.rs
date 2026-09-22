use clap::{Parser, Subcommand};
use imgsqueeze::{compress_image, resolve_jobs};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

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