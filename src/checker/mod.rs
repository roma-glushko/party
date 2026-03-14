pub mod runtime;
pub mod value;

use crate::compiler::CompiledProgram;

/// Run model checking on a compiled P program.
/// Runs multiple random iterations to find bugs.
pub fn check(program: &CompiledProgram) -> Result<(), String> {
    let _ = env_logger::try_init();
    let iterations = 20;

    for i in 0..iterations {
        let mut rt = runtime::Runtime::new(&program.programs);
        // On some iterations, bias unfair nondet toward true or false
        // to test liveness (can the system get stuck if $ always picks one branch?)
        rt.set_nondet_bias(match i % 4 {
            0 => None,          // random 50/50
            1 => Some(true),    // always true — test if system gets stuck
            2 => Some(false),   // always false — test if system gets stuck
            _ => None,          // random
        });
        if let Err(e) = rt.run() {
            return Err(e.message);
        }
    }

    Ok(())
}
