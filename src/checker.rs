use crate::compiler::CompiledProgram;

/// Run model checking / systematic testing on a compiled P program.
pub fn check(program: &CompiledProgram) -> Result<(), String> {
    // TODO: implement model checker
    let _ = program;
    todo!("implement P model checker")
}
