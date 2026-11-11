use anyhow::Result;
use clap::{Parser, Subcommand};
use ferrule::{add_modules, create_with_modules, doctor, plan_with_modules, preview_add};
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
        /// Optional module IDs, comma-separated. Only implemented modules can be selected.
        #[arg(long, value_delimiter = ',')]
        modules: Vec<String>,
        /// Print the generation plan without writing or running tools.
        #[arg(long)]
        dry_run: bool,
    },
    /// Add an optional module to a project created by this builder.
    Add {
        #[arg(value_delimiter = ',', num_args = 1..)]
        modules: Vec<String>,
        #[arg(long, default_value = ".")]
        project: PathBuf,
        /// Inspect exact changed file paths/hashes without writing or locking.
        #[arg(long)]
        dry_run: bool,
    },
    /// Resolve dependencies and regenerate Dart/Rust bindings.
    Sync {
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Roll back interrupted managed-file writes; never removes stale locks.
    Recover {
        #[arg(long, default_value = ".")]
        project: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Check Flutter, Rust and the pinned bridge generator.
    Doctor,
    /// List supported modules and their transport boundaries.
    Modules {
        #[arg(long)]
        json: bool,
    },
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
            mut modules,
            dry_run,
        } => {
            if ble {
                modules.push("ble".into());
            }
            if dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&plan_with_modules(&name, &directory, &modules)?)?
                );
            } else {
                let path = create_with_modules(&name, &directory, &modules)?;
                println!("Created {}", path.display());
                println!("Next: ferrule sync --project {}", path.display());
            }
        }
        Commands::Add {
            modules,
            project,
            dry_run,
        } => {
            if dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&preview_add(&project, &modules)?)?
                );
                return Ok(());
            }
            let preview = add_modules(&project, &modules)?;
            if !preview.added_modules.is_empty() {
                println!(
                    "Added {}. Run: ferrule sync --project {}",
                    preview.added_modules.join(", "),
                    project.display()
                );
            } else {
                println!("Selected modules are already installed; no files changed.");
            }
        }
        Commands::Sync { project } => ferrule::sync(&project)?,
        Commands::Recover { project, dry_run } => {
            let files = ferrule::recover(&project, dry_run)?;
            println!("{}", serde_json::to_string_pretty(&files)?);
        }
        Commands::Doctor => doctor()?,
        Commands::Modules { json } => {
            let catalog = ferrule::catalog::Catalog::embedded()?;
            let modules: Vec<_> = catalog.all().collect();
            if json {
                println!("{}", serde_json::to_string_pretty(&modules)?);
            } else {
                for module in modules {
                    let state = match module.availability {
                        ferrule::catalog::Availability::Available => "available",
                        ferrule::catalog::Availability::Planned => "planned",
                    };
                    println!(
                        "{} {} [{}] {}",
                        module.id, module.version, state, module.description
                    );
                }
            }
        }
    }
    Ok(())
}
