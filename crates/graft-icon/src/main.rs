//! Icon conversion utility for graft.
//!
//! Converts PNG icons to platform-specific formats:
//! - ICNS for macOS
//! - ICO for Windows

use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "graft-icon")]
#[command(about = "Convert PNG icons to platform-specific formats")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert PNG to macOS ICNS format
    Icns {
        /// Input PNG file
        input: PathBuf,
        /// Output ICNS file
        output: PathBuf,
    },
    /// Convert PNG to Windows ICO format
    Ico {
        /// Input PNG file
        input: PathBuf,
        /// Output ICO file
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Icns { input, output } => convert_to_icns(&input, &output),
        Commands::Ico { input, output } => convert_to_ico(&input, &output),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Convert PNG to macOS ICNS format.
fn convert_to_icns(input: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let file = File::open(input)
        .map_err(|e| format!("Failed to open input file: {}", e))?;
    let reader = BufReader::new(file);

    let image = icns::Image::read_png(reader)
        .map_err(|e| format!("Failed to read PNG: {}", e))?;

    let mut icon_family = icns::IconFamily::new();
    icon_family.add_icon(&image)
        .map_err(|e| format!("Failed to add icon: {}", e))?;

    let output_file = File::create(output)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    icon_family.write(output_file)
        .map_err(|e| format!("Failed to write ICNS: {}", e))?;

    println!("Created {}", output.display());
    Ok(())
}

/// Convert PNG to Windows ICO format with multiple sizes.
fn convert_to_ico(input: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let img = image::open(input)
        .map_err(|e| format!("Failed to load PNG: {}", e))?;

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    // Create icons at multiple sizes for best display
    for size in [256, 128, 64, 48, 32, 16] {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();
        let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
        icon_dir.add_entry(
            ico::IconDirEntry::encode(&icon_image)
                .map_err(|e| format!("Failed to encode icon at size {}: {}", size, e))?
        );
    }

    let file = File::create(output)
        .map_err(|e| format!("Failed to create output file: {}", e))?;
    icon_dir.write(BufWriter::new(file))
        .map_err(|e| format!("Failed to write ICO: {}", e))?;

    println!("Created {}", output.display());
    Ok(())
}
