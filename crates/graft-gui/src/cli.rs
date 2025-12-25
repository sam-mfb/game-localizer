use crate::runner::{PatchRunner, ProgressEvent};
use crate::validator::PatchValidator;
use std::io::{self, Write};
use std::path::Path;

/// Run in headless (CLI) mode with embedded patch data
pub fn run_headless(
    patch_data: &[u8],
    target_path: &Path,
    skip_confirm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Graft Patcher - Headless Mode");
    println!("==============================");

    // Validate patch and get info
    print!("Validating patch data... ");
    io::stdout().flush()?;

    let info = PatchValidator::validate(patch_data)?;
    println!("done");

    // Show patch info
    println!("\nPatch Information:");
    println!("  Version: {}", info.version);
    println!("  Operations: {}", info.entry_count);
    println!("    - {} patches", info.patches);
    println!("    - {} additions", info.additions);
    println!("    - {} deletions", info.deletions);
    println!("\nTarget: {}", target_path.display());

    // Confirm unless -y flag
    if !skip_confirm {
        print!("\nApply patch? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Create runner and apply patch
    println!("\nApplying patch...");

    let runner = PatchRunner::new(patch_data)?;
    let result = runner.apply(target_path, |event| {
        match event {
            ProgressEvent::Processing { file, index, total } => {
                print!("  [{}/{}] {}... ", index + 1, total, file);
                let _ = io::stdout().flush();
            }
            ProgressEvent::Processed { .. } => {
                println!("ok");
            }
            ProgressEvent::Done { .. } => {}
            ProgressEvent::Error { .. } => {
                println!("FAILED");
            }
        }
    });

    match result {
        Ok(()) => {
            println!("\nPatch applied successfully!");
            Ok(())
        }
        Err(e) => {
            eprintln!("\nError: {}", e);
            std::process::exit(1);
        }
    }
}
