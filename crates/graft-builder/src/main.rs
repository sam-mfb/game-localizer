use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "graft-builder")]
#[command(about = "Build self-contained GUI patchers from graft patches")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a GUI patcher executable from a patch directory
    Build {
        /// Path to the patch directory (containing manifest.json)
        patch_dir: PathBuf,

        /// Output directory for the built executable
        #[arg(short, long, default_value = "./dist")]
        output: PathBuf,

        /// Name for the patcher executable (without extension)
        #[arg(short, long)]
        name: Option<String>,

        /// Cross-compile for specific targets (comma-separated)
        /// Available: linux-x64, linux-arm64, windows
        /// Requires: Docker and `cargo install cross`
        #[arg(long, value_delimiter = ',')]
        targets: Option<Vec<String>>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            patch_dir,
            output,
            name,
            targets,
        } => {
            let result = match targets {
                Some(ref target_names) => {
                    // Cross-compilation mode
                    match graft_builder::targets::parse_targets(target_names) {
                        Ok(parsed_targets) => graft_builder::build_cross(
                            &patch_dir,
                            &output,
                            name.as_deref(),
                            &parsed_targets,
                        ),
                        Err(e) => Err(e),
                    }
                }
                None => {
                    // Native build mode
                    graft_builder::build(&patch_dir, &output, name.as_deref()).map(|p| vec![p])
                }
            };

            match result {
                Ok(output_paths) => {
                    for path in &output_paths {
                        println!("Built patcher: {}", path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
    }
}
