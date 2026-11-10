use anyhow::Result;
use clap::{Parser, Subcommand};
use ferrule::{add_ble, create, doctor, plan};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ferrule",
    version,
    about = "Flutter + Rust modular app builder"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create an iOS/Android app through the official FRB generator.
    New {
        /// A lowercase snake_case Dart package name.
        name: String,
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        ble: bool,
        /// Print the generation plan without writing or running tools.
        #[arg(long)]
        dry_run: bool,
    },
    /// Add an optional module to a project created by this builder.
    Add {
        #[arg(value_parser = ["ble"])]
        module: String,
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Resolve dependencies and regenerate Dart/Rust bindings.
    Sync {
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Check Flutter, Rust and the pinned bridge generator.
    Doctor,
    /// List supported modules and their transport boundaries.
    Modules,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Commands::New {
            name,
            directory,
            ble,
            dry_run,
        } => {
            if dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&plan(&name, &directory, ble)?)?
                );
            } else {
                let path = create(&name, &directory, ble)?;
                println!("Created {}", path.display());
                println!("Next: ferrule sync --project {}", path.display());
            }
        }
        Commands::Add { module: _, project } => {
            if add_ble(&project)? {
                println!(
                    "BLE added. Run: ferrule sync --project {}",
                    project.display()
                );
            } else {
                println!("BLE is already installed; no files changed.");
            }
        }
        Commands::Sync { project } => ferrule::sync(&project)?,
        Commands::Doctor => doctor()?,
        Commands::Modules => println!(
            "core  Flutter UI + Rust domain through flutter_rust_bridge\nble   Optional iOS/Android central scan; Flutter native transport + Rust packet decoding"
        ),
    }
    Ok(())
}
