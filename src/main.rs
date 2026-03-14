use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "plang", about = "P language compiler and model checker")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile a P program
    Compile {
        /// Path to directory containing .p files
        path: PathBuf,
    },
    /// Compile and run model checking on a P program
    Check {
        /// Path to directory containing .p files
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { path } => {
            match plang::compiler::compile(&path) {
                Ok(_) => println!("Compilation successful."),
                Err(errors) => {
                    for e in &errors {
                        eprintln!("error: {}", e.message);
                    }
                    std::process::exit(1);
                }
            }
        }
        Command::Check { path } => {
            let program = plang::compiler::compile(&path).unwrap_or_else(|errors| {
                for e in &errors {
                    eprintln!("error: {}", e.message);
                }
                std::process::exit(1);
            });
            match plang::checker::check(&program) {
                Ok(()) => println!("Model checking passed."),
                Err(msg) => {
                    eprintln!("Violation: {msg}");
                    std::process::exit(1);
                }
            }
        }
    }
}
