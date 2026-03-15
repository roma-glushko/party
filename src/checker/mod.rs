pub mod runtime;
pub mod scheduler;
pub mod trace;
pub mod value;

use crate::compiler::CompiledProgram;

/// Result of model checking, including trace on failure.
pub struct CheckResult {
    pub ok: bool,
    pub error: Option<String>,
    pub trace: Vec<trace::TraceEvent>,
    pub schedule: Option<trace::Schedule>,
}

/// Run model checking on a compiled P program.
/// Uses a combination of DFS systematic exploration and random scheduling.
pub fn check(program: &CompiledProgram) -> Result<(), String> {
    let result = check_with_trace(program);
    if result.ok {
        Ok(())
    } else {
        Err(result.error.unwrap_or_default())
    }
}

/// Run model checking and return structured result with trace.
pub fn check_with_trace(program: &CompiledProgram) -> CheckResult {
    let _ = env_logger::try_init();

    // Phase 1: DFS systematic exploration
    let max_dfs_iterations = 100;
    let mut dfs = scheduler::DfsScheduler::new(500);

    for i in 0..max_dfs_iterations {
        let mut rt = runtime::Runtime::new(&program.programs);
        rt.set_dfs_scheduler(dfs);
        if let Err(e) = rt.run() {
            return CheckResult {
                ok: false,
                error: Some(e.message),
                trace: rt.tracer.events().to_vec(),
                schedule: Some(rt.get_schedule()),
            };
        }
        dfs = rt.take_dfs_scheduler().unwrap();

        if !dfs.prepare_for_next_iteration() {
            log::debug!("DFS exhausted search space after {} iterations", i + 1);
            break;
        }
    }

    // Phase 2: Random iterations with bias
    for i in 0..30 {
        let mut rt = runtime::Runtime::new(&program.programs);
        rt.set_nondet_bias(match i % 4 {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        });
        if let Err(e) = rt.run() {
            return CheckResult {
                ok: false,
                error: Some(e.message),
                trace: rt.tracer.events().to_vec(),
                schedule: Some(rt.get_schedule()),
            };
        }
    }

    CheckResult {
        ok: true,
        error: None,
        trace: Vec::new(),
        schedule: None,
    }
}
