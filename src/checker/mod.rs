pub mod runtime;
pub mod value;

use crate::compiler::CompiledProgram;

/// Run model checking on a compiled P program.
/// Runs multiple random iterations to find bugs.
pub fn check(program: &CompiledProgram) -> Result<(), String> {
    let iterations = 10; // Enough for most nondeterministic bugs

    for _ in 0..iterations {
        let mut rt = runtime::Runtime::new(&program.programs);
        if let Err(e) = rt.run() {
            return Err(e.message);
        }
    }

    Ok(())
}
