use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

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
    /// Format .p source files (writes in place by default)
    Format {
        /// Path to .p file or directory containing .p files
        path: PathBuf,

        /// Only check formatting without modifying files (exit 1 if unformatted)
        #[arg(long = "check")]
        check: bool,
    },
    /// Compile and run model checking on a P program
    Check {
        /// Path to directory or .pproj file containing .p files
        path: PathBuf,

        /// Test case name to check (if multiple test cases exist)
        #[arg(short = 't', long = "testcase", alias = "tc")]
        testcase: Option<String>,

        /// Number of scheduling iterations
        #[arg(short = 'i', long = "iterations", default_value = "100")]
        iterations: usize,

        /// Maximum scheduling steps per iteration
        #[arg(short = 's', long = "max-steps", default_value = "10000")]
        max_steps: usize,

        /// Scheduling strategy (random, dfs)
        #[arg(long = "strategy", default_value = "random")]
        strategy: String,
    },
}

const SEPARATOR: &str = "----------------------------------------";

fn main() {
    let _ = env_logger::try_init();
    let cli = Cli::parse();

    match cli.command {
        Command::Format { path, check } => {
            run_format(&path, check);
        }
        Command::Compile { path } => {
            run_compile(&path);
            println!("\n~~ [PLang]: Thanks for using P! ~~");
        }
        Command::Check { path, testcase, iterations, max_steps, strategy } => {
            let program = run_compile(&path);

            // Discover test cases from the program
            let test_cases = discover_test_cases(&program);

            println!("{SEPARATOR}");
            if test_cases.is_empty() {
                println!(".. Checking {}", path.display());
            } else if test_cases.len() == 1 || testcase.is_some() {
                let tc_name = testcase.as_deref()
                    .unwrap_or_else(|| &test_cases[0]);
                println!(".. Checking test case: {tc_name}");
            } else {
                // Multiple test cases, no specific one selected
                eprintln!(
                    "Error: We found '{}' test cases. Please provide a more precise name of the \
                     test case you wish to check using (--testcase | -tc).",
                    test_cases.len()
                );
                println!("Possible options are:");
                for tc in &test_cases {
                    println!("  {tc}");
                }
                println!("\n~~ [PLang]: Thanks for using P! ~~");
                std::process::exit(1);
            }

            let start = Instant::now();
            println!("Starting model checking ...");
            println!(
                "  Strategy: {strategy}, Iterations: {iterations}, Max steps: {max_steps}"
            );

            match plang::checker::check(&program) {
                Ok(()) => {
                    let elapsed = start.elapsed();
                    println!("... Model checking completed in {:.2}s", elapsed.as_secs_f64());
                    println!("... Found 0 bugs.");
                }
                Err(msg) => {
                    let elapsed = start.elapsed();
                    println!("... Model checking completed in {:.2}s", elapsed.as_secs_f64());
                    eprintln!("... Found a bug.");
                    eprintln!("Error: {msg}");
                    println!("{SEPARATOR}");
                    println!("\n~~ [PLang]: Thanks for using P! ~~");
                    std::process::exit(1);
                }
            }
            println!("{SEPARATOR}");
            println!("\n~~ [PLang]: Thanks for using P! ~~");
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
    let start = Instant::now();
    match plang::compiler::compile(&dir) {
        Ok(program) => {
            let elapsed = start.elapsed();
            println!(
                "Code generation ...  [done in {:.2}s]",
                elapsed.as_secs_f64()
            );
            println!("{SEPARATOR}");
            program
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e}");
            }
            println!("{SEPARATOR}");
            println!("\n~~ [PLang]: Thanks for using P! ~~");
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
                    println!(
                        ".. Searching for a P project file *.pproj locally in the current folder"
                    );
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

fn run_format(path: &PathBuf, check: bool) {
    let files = collect_p_files(path);
    if files.is_empty() {
        eprintln!("No .p files found at {}", path.display());
        std::process::exit(1);
    }

    let mut unformatted = Vec::new();
    let mut formatted_count = 0;

    for file in &files {
        let source = std::fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading {}: {e}", file.display());
            std::process::exit(1);
        });

        // Parse
        let tokens = match plang::compiler::lexer::lex(&source) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Lex error in {}: {e}", file.display());
                std::process::exit(1);
            }
        };
        let mut parser = plang::compiler::parser::Parser::new(tokens, source.clone());
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Parse error in {}: {e}", file.display());
                std::process::exit(1);
            }
        };

        let formatted = plang::compiler::formatter::format_program(&program);

        if formatted != source {
            if check {
                unformatted.push(file.clone());
            } else {
                std::fs::write(file, &formatted).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {e}", file.display());
                    std::process::exit(1);
                });
                println!("Formatted {}", file.display());
                formatted_count += 1;
            }
        }
    }

    if check {
        if unformatted.is_empty() {
            println!("All {} files are formatted.", files.len());
        } else {
            eprintln!("The following files need formatting:");
            for f in &unformatted {
                eprintln!("  {}", f.display());
            }
            std::process::exit(1);
        }
    } else if formatted_count == 0 {
        println!("All {} files already formatted.", files.len());
    } else {
        println!("Formatted {} of {} files.", formatted_count, files.len());
    }
}

fn collect_p_files(path: &PathBuf) -> Vec<PathBuf> {
    if path.is_file() && path.extension().is_some_and(|e| e == "p") {
        return vec![path.clone()];
    }
    if path.is_dir() {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(path).unwrap().flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().is_some_and(|e| e == "p") {
                files.push(p);
            }
        }
        files.sort();
        return files;
    }
    Vec::new()
}

/// Extract test case names from the compiled program's test declarations.
fn discover_test_cases(program: &plang::compiler::CompiledProgram) -> Vec<String> {
    let mut names = Vec::new();
    for prog in &program.programs {
        for decl in &prog.decls {
            if let plang::compiler::ast::TopDecl::TestDecl(t) = decl {
                names.push(t.name.clone());
            }
        }
    }
    names
}
