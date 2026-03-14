pub mod runtime;
pub mod scheduler;
pub mod value;

use crate::compiler::CompiledProgram;

/// Run model checking on a compiled P program.
/// Uses a combination of DFS systematic exploration and random scheduling.
pub fn check(program: &CompiledProgram) -> Result<(), String> {
    let _ = env_logger::try_init();

    // Phase 1: DFS systematic exploration (finds most bugs deterministically)
    let max_dfs_iterations = 100;
    let mut dfs = scheduler::DfsScheduler::new(500);

    for i in 0..max_dfs_iterations {
        let mut rt = runtime::Runtime::new(&program.programs);
        rt.set_dfs_scheduler(dfs);
        if let Err(e) = rt.run() {
            return Err(e.message);
        }
        dfs = rt.take_dfs_scheduler().unwrap();

        if !dfs.prepare_for_next_iteration() {
            log::debug!("DFS exhausted search space after {} iterations", i + 1);
            break;
        }
    }

    // Phase 2: Random iterations with bias (catches scheduling-sensitive bugs)
    for i in 0..10 {
        let mut rt = runtime::Runtime::new(&program.programs);
        rt.set_nondet_bias(match i % 4 {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        });
        if let Err(e) = rt.run() {
            return Err(e.message);
        }
    }

    Ok(())
}
