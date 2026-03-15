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
        /// Path to directory or .pproj file containing .p files
        path: PathBuf,
    },
    /// Compile and run model checking on a P program
    Check {
        /// Path to directory or .pproj file containing .p files
        path: PathBuf,
    },
}

const SEPARATOR: &str = "----------------------------------------";

fn main() {
    let _ = env_logger::try_init();
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { path } => {
            run_compile(&path);
        }
        Command::Check { path } => {
            let program = run_compile(&path);
            println!("{SEPARATOR}");
            println!("Model checking ...");
            match plang::checker::check(&program) {
                Ok(()) => {
                    println!("Model checking passed.");
                }
                Err(msg) => {
                    eprintln!("Error: {msg}");
                    println!("{SEPARATOR}");
                    println!("~~ [PLang]: Thanks for using P! ~~");
                    std::process::exit(1);
                }
            }
            println!("{SEPARATOR}");
            println!("~~ [PLang]: Thanks for using P! ~~");
        }
    }
}

fn run_compile(path: &PathBuf) -> plang::compiler::CompiledProgram {
    let dir = resolve_project_path(path);
    println!("{SEPARATOR}");
    println!("==== Loading project: {}", dir.display());

    // List .p files
    if dir.is_dir() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "p") {
                println!("....... includes p file: {}", p.display());
            }
        }
    }
    println!("{SEPARATOR}");

    println!("Parsing ...");
    println!("Type checking ...");
    match plang::compiler::compile(&dir) {
        Ok(program) => {
            println!("Compilation successful.");
            println!("{SEPARATOR}");
            program
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e}");
            }
            println!("{SEPARATOR}");
            println!("~~ [PLang]: Thanks for using P! ~~");
            std::process::exit(1);
        }
    }
}

fn resolve_project_path(path: &PathBuf) -> PathBuf {
    if path.is_dir() {
        // Look for .pproj file in directory
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "pproj") {
                    println!(".. Found P project file: {}", p.display());
                    return path.clone();
                }
            }
        }
        path.clone()
    } else if path.extension().is_some_and(|e| e == "pproj") {
        println!(".. Found P project file: {}", path.display());
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.clone()
    }
}
