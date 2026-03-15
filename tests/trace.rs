use std::path::Path;

fn compile_and_check(source: &str) -> (bool, Vec<String>) {
    let tokens = plang::compiler::lexer::lex(source).expect("lex failed");
    let mut parser = plang::compiler::parser::Parser::new(tokens, source.to_string());
    let program = parser.parse_program().expect("parse failed");
    let programs = vec![program];
    // Skip type checking for trace tests — go straight to runtime
    let compiled = plang::compiler::CompiledProgram { programs };

    let mut rt = plang::checker::runtime::Runtime::new(&compiled.programs);
    let result = rt.run();
    let trace = rt.get_trace();
    (result.is_ok(), trace)
}

#[test]
fn trace_records_machine_creation() {
    let (ok, trace) = compile_and_check(r#"
        machine Main { start state S { entry { } } }
    "#);
    assert!(ok);
    assert!(trace.iter().any(|e| e.contains("create") && e.contains("Main")),
        "trace should record machine creation, got: {trace:?}");
}

#[test]
fn trace_records_state_transitions() {
    let (ok, trace) = compile_and_check(r#"
        event E;
        machine Main {
            start state A {
                entry { send this, E; }
                on E goto B;
            }
            state B { }
        }
    "#);
    assert!(ok);
    assert!(trace.iter().any(|e| e.contains("Main") && e.contains("->") && e.contains("B")),
        "trace should record state transition, got: {trace:?}");
}

#[test]
fn trace_records_send_events() {
    let (ok, trace) = compile_and_check(r#"
        event Ping: machine;
        machine Main {
            var other: machine;
            start state S {
                entry { other = new Worker(); send other, Ping, this; }
            }
        }
        machine Worker {
            start state S { on Ping do (payload: machine) { } }
        }
    "#);
    assert!(ok);
    assert!(trace.iter().any(|e| e.contains("send") && e.contains("Ping")),
        "trace should record send event, got: {trace:?}");
}

#[test]
fn trace_records_assertion_failure() {
    let (ok, trace) = compile_and_check(r#"
        machine Main {
            start state S {
                entry { assert false, "boom"; }
            }
        }
    "#);
    assert!(!ok, "should find assertion failure");
    assert!(trace.iter().any(|e| e.contains("assert") || e.contains("boom")),
        "trace should record assertion failure, got: {trace:?}");
}

#[test]
fn trace_records_raise_event() {
    let (ok, trace) = compile_and_check(r#"
        event E;
        machine Main {
            start state A {
                entry { raise E; }
                on E goto B;
            }
            state B { }
        }
    "#);
    assert!(ok);
    assert!(trace.iter().any(|e| e.contains("raise") && e.contains("E")),
        "trace should record raise event, got: {trace:?}");
}

#[test]
fn trace_records_unhandled_event() {
    let (ok, trace) = compile_and_check(r#"
        event E;
        event F;
        machine Main {
            start state S {
                entry { send this, F; }
                on E do { }
            }
        }
    "#);
    assert!(!ok, "should find unhandled event");
    assert!(trace.iter().any(|e| e.contains("unhandled")),
        "trace should record unhandled event, got: {trace:?}");
}

#[test]
fn trace_ends_with_error_on_failure() {
    let (ok, trace) = compile_and_check(r#"
        machine Main {
            start state S {
                entry { assert 1 == 2; }
            }
        }
    "#);
    assert!(!ok);
    let last = trace.last().expect("trace should not be empty");
    assert!(last.contains("ERROR") || last.contains("assert"),
        "last trace entry should be the error, got: {last}");
}

#[test]
fn trace_shows_step_numbers() {
    let (ok, trace) = compile_and_check(r#"
        event E;
        machine Main {
            start state A {
                entry { send this, E; }
                on E goto B;
            }
            state B { }
        }
    "#);
    assert!(ok);
    // At least some trace entries should have a step/sequence number
    assert!(trace.iter().any(|e| e.starts_with('<') || e.contains('#') || e.chars().next().map_or(false, |c| c.is_ascii_digit())),
        "trace should include step markers, got: {trace:?}");
}

#[test]
fn trace_is_empty_for_trivial_program() {
    let (ok, trace) = compile_and_check(r#"
        machine Main { start state S { } }
    "#);
    assert!(ok);
    // Should still have at least a creation entry
    assert!(!trace.is_empty(), "trace should not be empty even for trivial program");
}

#[test]
fn trace_multi_machine_shows_both() {
    let (ok, trace) = compile_and_check(r#"
        event Ping;
        machine Main {
            start state S { entry { new Worker(); } }
        }
        machine Worker {
            start state S { }
        }
    "#);
    assert!(ok);
    assert!(trace.iter().any(|e| e.contains("Main")), "trace should mention Main");
    assert!(trace.iter().any(|e| e.contains("Worker")), "trace should mention Worker");
}
