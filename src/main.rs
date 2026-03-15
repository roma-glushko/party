use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "party",
    version,
    about = "Party — P lAnguage in RusT, formal verification for distributed systems",
    long_about = "Party is a Rust implementation of the P language compiler and model checker.\n\
                  P is a state-machine based language for modeling and verifying\n\
                  complex distributed systems.\n\n\
                  Commands:\n  \
                    lint    — Check .p files for syntax and type errors\n  \
                    format  — Auto-format .p source files\n  \
                    verify  — Run model checking to find concurrency bugs\n\n\
                  Examples:\n  \
                    party lint myproject/\n  \
                    party format src/ --check\n  \
                    party verify myproject/ -t TestName\n  \
                    party verify myproject/ --replay bug.prun",
    after_help = "See https://p-org.github.io/P/ for P language documentation."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check .p files for parse errors and type errors.
    ///
    /// Runs the parser and type checker on all .p files in the given
    /// directory. Reports errors with file, line, and column info.
    /// Exit code 0 if no errors, 1 if errors found.
    Lint {
        /// Directory containing .p files, or a .pproj project file
        path: PathBuf,
    },

    /// Auto-format .p source files with consistent style.
    ///
    /// By default, formats files in place. Use --check to verify
    /// formatting without modifying files (useful in CI).
    ///
    /// Style: 2-space indent, consistent spacing, blank lines
    /// between state handlers, short bodies on one line.
    Format {
        /// .p file or directory containing .p files
        path: PathBuf,

        /// Check formatting without modifying files.
        /// Exit code 0 if all formatted, 1 if changes needed.
        #[arg(long = "check")]
        check: bool,
    },

    /// Run formal verification (model checking) on a P program.
    ///
    /// Systematically explores the state space of the program to
    /// find assertion violations, deadlocks, and liveness bugs.
    /// Uses DFS exploration followed by randomized scheduling.
    ///
    /// On finding a bug, prints a counterexample trace and saves
    /// the schedule to a .prun file for deterministic replay.
    ///
    /// Examples:
    ///   party verify myproject/
    ///   party verify myproject/ -t TestSafety
    ///   party verify myproject/ --replay bug.prun
    Verify {
        /// Directory containing .p files, or a .pproj project file
        path: PathBuf,

        /// Name of the test case to verify.
        /// Required when the program defines multiple test cases.
        /// Use without this flag to list available test cases.
        #[arg(short = 't', long = "testcase", alias = "tc")]
        testcase: Option<String>,

        /// Number of scheduling iterations to explore
        #[arg(short = 'i', long = "iterations", default_value = "100")]
        iterations: usize,

        /// Maximum scheduling steps per iteration
        #[arg(short = 's', long = "max-steps", default_value = "10000")]
        max_steps: usize,

        /// Scheduling strategy: 'random' or 'dfs'
        #[arg(long = "strategy", default_value = "random")]
        strategy: String,

        /// Replay a saved schedule file (.prun) to reproduce a bug
        /// deterministically. Schedule files are auto-saved when
        /// a bug is found.
        #[arg(long = "replay")]
        replay: Option<PathBuf>,
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
        Command::Lint { path } => {
            run_compile(&path);
        }
        Command::Verify { path, testcase, iterations, max_steps, strategy, replay } => {
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
                    std::process::exit(1);
            }

            let start = Instant::now();
            let result = if let Some(ref replay_path) = replay {
                println!("Replaying schedule from {} ...", replay_path.display());
                let schedule = party::checker::trace::Schedule::load(replay_path)
                    .unwrap_or_else(|e| { eprintln!("Error loading schedule: {e}"); std::process::exit(1); });
                let mut rt = party::checker::runtime::Runtime::new(&program.programs);
                rt.set_schedule(schedule);
                let run_result = rt.run();
                party::checker::CheckResult {
                    ok: run_result.is_ok(),
                    error: run_result.err().map(|e| e.message),
                    trace: rt.tracer.events().to_vec(),
                    schedule: Some(rt.get_schedule()),
                }
            } else {
                println!("Starting model checking ...");
                println!("  Strategy: {strategy}, Iterations: {iterations}, Max steps: {max_steps}");
                party::checker::check_with_trace(&program)
            };
            let elapsed = start.elapsed();
            println!("... Model checking completed in {:.2}s", elapsed.as_secs_f64());
            if result.ok {
                println!("... Found 0 bugs.");
            } else {
                eprintln!("... Found a bug.");
                if let Some(ref msg) = result.error {
                    eprintln!("Error: {msg}");
                }
                // Print execution trace for debugging
                if !result.trace.is_empty() {
                    eprintln!();
                    let trace_len = result.trace.len();
                    let show = 30;
                    let start_idx = if trace_len > show { trace_len - show } else { 0 };
                    eprintln!("=== Counterexample Trace (last {} of {} steps) ===",
                        trace_len - start_idx, trace_len);
                    if start_idx > 0 {
                        eprintln!("  ... ({start_idx} earlier steps omitted)");
                    }
                    for event in &result.trace[start_idx..] {
                        eprintln!("  {event}");
                    }
                    eprintln!("=== End Trace ===");
                }
                // Save schedule for replay (skip if already replaying)
                if replay.is_none() {
                    if let Some(ref sched) = result.schedule {
                        let sched_path = path.with_extension("prun");
                        if let Err(e) = sched.save(&sched_path) {
                            eprintln!("Warning: could not save schedule: {e}");
                        } else {
                            eprintln!("Schedule saved to: {}", sched_path.display());
                            eprintln!("Replay with: party verify {} --replay {}", path.display(), sched_path.display());
                        }
                    }
                }
                println!("{SEPARATOR}");
                std::process::exit(1);
            }
            println!("{SEPARATOR}");
        }
    }
}

fn run_compile(path: &PathBuf) -> party::compiler::CompiledProgram {
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
    match party::compiler::compile(&dir) {
        Ok(program) => {
            let elapsed = start.elapsed();
            println!("No errors found.  [done in {:.2}s]", elapsed.as_secs_f64());
            println!("{SEPARATOR}");
            program
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("error: {e}");
            }
            println!("{SEPARATOR}");
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
        let tokens = match party::compiler::lexer::lex(&source) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Lex error in {}: {e}", file.display());
                std::process::exit(1);
            }
        };
        let mut parser = party::compiler::parser::Parser::new(tokens, source.clone());
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Parse error in {}: {e}", file.display());
                std::process::exit(1);
            }
        };

        let formatted = party::compiler::formatter::format_program(&program);

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
fn discover_test_cases(program: &party::compiler::CompiledProgram) -> Vec<String> {
    let mut names = Vec::new();
    for prog in &program.programs {
        for decl in &prog.decls {
            if let party::compiler::ast::TopDecl::TestDecl(t) = decl {
                names.push(t.name.clone());
            }
        }
    }
    names
}
