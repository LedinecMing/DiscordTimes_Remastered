//! lit2png — Discord Times (2003) LIT/UGS/TGA/BMP graphics → PNG converter.
//!
//! Format reference: notes/RUST_CONVERTER_GUIDE.md.

use clap::Parser;
use lit2png_lib::{error::{LitError, Result}, lit, pngout, ugs};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "lit2png", version, about = "LIT/UGS/TGA/BMP → PNG converter for Discord Times assets")]
struct Cli {
    /// Input files or directories (recursed for image formats)
    #[arg(required = true)]
    input: Vec<PathBuf>,
    /// Output directory
    #[arg(short, long, default_value = "out")]
    out: PathBuf,
    /// Which UGS frames to export: "all" or a single frame index
    #[arg(long, default_value = "all")]
    frames: String,
    /// NEAREST upscale factor for UGS previews (1 = native)
    #[arg(long, default_value = "1")]
    upscale: u32,
    /// Report per-file result lines (default: summary only)
    #[arg(long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        eprintln!("lit2png: error: {e}");
        std::process::exit(1);
    }
}

struct Stats {
    lit: usize,
    ugs: usize,
    ugs_frames: usize,
    raster: usize,
    skipped: usize,
    failed: Vec<(PathBuf, String)>,
}

fn run(cli: &Cli) -> Result<()> {
    std::fs::create_dir_all(&cli.out)?;
    let mut files = Vec::new();
    for input in &cli.input {
        collect_inputs(input, &mut files)?;
    }
    files.sort();
    println!("{} input file(s)", files.len());

    let mut stats = Stats { lit: 0, ugs: 0, ugs_frames: 0, raster: 0, skipped: 0, failed: Vec::new() };
    let t0 = Instant::now();
    for path in &files {
        match convert_one(path, &cli.out, cli) {
            Ok(Converted::Lit { alpha_mask }) => {
                stats.lit += 1;
                if cli.verbose {
                    println!("  [lit] {}{}", path.display(), if alpha_mask { " (+alpha mask)" } else { "" });
                }
            }
            Ok(Converted::Ugs { frames }) => {
                stats.ugs += 1;
                stats.ugs_frames += frames;
                if cli.verbose {
                    println!("  [ugs] {} ({} frames)", path.display(), frames);
                }
            }
            Ok(Converted::Raster) => {
                stats.raster += 1;
                if cli.verbose {
                    println!("  [raster] {}", path.display());
                }
            }
            Ok(Converted::Skipped(reason)) => {
                stats.skipped += 1;
                if cli.verbose {
                    println!("  [skip] {} ({})", path.display(), reason);
                }
            }
            Err(e) => {
                stats.failed.push((path.clone(), e.to_string()));
                eprintln!("  [FAIL] {}: {e}", path.display());
            }
        }
    }
    let dt = t0.elapsed().as_secs_f64();
    println!(
        "done in {dt:.1}s: {} lit, {} ugs ({} frames), {} raster, {} skipped, {} failed",
        stats.lit, stats.ugs, stats.ugs_frames, stats.raster, stats.skipped, stats.failed.len()
    );
    if !stats.failed.is_empty() {
        for (p, e) in &stats.failed {
            eprintln!("FAILED: {}: {e}", p.display());
        }
        std::process::exit(2);
    }
    Ok(())
}

enum Converted {
    Lit { alpha_mask: bool },
    Ugs { frames: usize },
    Raster,
    Skipped(&'static str),
}

fn collect_inputs(input: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if input.is_dir() {
        let mut entries = std::fs::read_dir(input)?.collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let p = entry.path();
            if p.is_dir() {
                collect_inputs(&p, out)?;
            } else {
                out.push(p);
            }
        }
    } else {
        out.push(input.to_path_buf());
    }
    Ok(())
}

fn ext_lower(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn convert_one(path: &Path, out_dir: &Path, cli: &Cli) -> Result<Converted> {
    let ext = ext_lower(path);
    match ext.as_str() {
        "lit" => convert_lit(path, out_dir),
        "ugs" => convert_ugs(path, out_dir, cli),
        "tga" | "bmp" => convert_raster(path, out_dir),
        "spi" => Ok(Converted::Skipped("editor sprite index")),
        "cfg" | "ini" => Ok(Converted::Skipped("config")),
        _ => {
            // Sniff: a LIT\0 magic converts even with a wrong extension.
            let magic = read_prefix(path, 4)?;
            if magic == b"LIT\0" {
                convert_lit(path, out_dir)
            } else {
                Ok(Converted::Skipped("unknown format"))
            }
        }
    }
}

fn read_prefix(path: &Path, n: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; n];
    let mut read = 0;
    while read < n {
        let k = f.read(&mut buf[read..])?;
        if k == 0 {
            buf.truncate(read);
            break;
        }
        read += k;
    }
    Ok(buf)
}

fn stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn convert_lit(path: &Path, out_dir: &Path) -> Result<Converted> {
    let buf = std::fs::read(path)?;
    let img = lit::decode(&buf)?;
    let out_path = out_dir.join(format!("{}.png", stem(path)));
    pngout::save_rgba(&out_path, &img)?;
    let mut alpha_mask = false;
    if img.alpha.is_some() {
        let mask_path = out_dir.join(format!("{}_alpha.png", stem(path)));
        pngout::save_alpha_mask(&mask_path, &img)?;
        alpha_mask = true;
    }
    Ok(Converted::Lit { alpha_mask })
}

fn convert_ugs(path: &Path, out_dir: &Path, cli: &Cli) -> Result<Converted> {
    let buf = std::fs::read(path)?;
    let frames = ugs::decode_ugs(&buf)?;
    let total = frames.len();
    let indices: Vec<usize> = match cli.frames.as_str() {
        "all" => (0..total).collect(),
        n => vec![n.parse::<usize>().map_err(|_| LitError::Other(format!("bad --frames {n}")))?],
    };
    for i in indices {
        let Some(frame) = frames.get(i) else { continue };
        let (name, idx) = match &frame.kind {
            // Atlas record: carry (kind,id) in the name — kind groups animation
            // phases of the same object id.
            Some((kind, id)) => (format!("{}_k{}", id, kind), None),
            None => (stem(path), Some(i)),
        };
        let out_path = match idx {
            Some(0) if total == 1 => out_dir.join(format!("{name}.png")),
            Some(i) => out_dir.join(format!("{name}_frame{i:03}.png")),
            None => out_dir.join(format!("{name}.png")),
        };
        if cli.upscale > 1 {
            pngout::save_rgba_scaled(&out_path, &frame.image, cli.upscale)?;
        } else {
            pngout::save_rgba(&out_path, &frame.image)?;
        }
    }
    Ok(Converted::Ugs { frames: total })
}

fn convert_raster(path: &Path, out_dir: &Path) -> Result<Converted> {
    let img = image::open(path)?;
    let out_path = out_dir.join(format!("{}.png", stem(path)));
    img.save_with_format(out_path, image::ImageFormat::Png)?;
    Ok(Converted::Raster)
}
