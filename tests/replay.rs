use std::path::Path;

fn compile_programs(source: &str) -> party::compiler::CompiledProgram {
    let tokens = party::compiler::lexer::lex(source).expect("lex");
    let mut parser = party::compiler::parser::Parser::new(tokens, source.to_string());
    let program = parser.parse_program().expect("parse");
    party::compiler::CompiledProgram { programs: vec![program] }
}

#[test]
fn schedule_roundtrip_serialization() {
    // A schedule can be serialized to string and deserialized back
    let schedule = party::checker::trace::Schedule {
        scheduling_choices: vec![0, 1, 0, 2],
        nondet_choices: vec![true, false, true],
    };
    let serialized = schedule.to_string();
    let deserialized = party::checker::trace::Schedule::parse(&serialized).unwrap();
    assert_eq!(schedule.scheduling_choices, deserialized.scheduling_choices);
    assert_eq!(schedule.nondet_choices, deserialized.nondet_choices);
}

#[test]
fn schedule_save_and_load_file() {
    let schedule = party::checker::trace::Schedule {
        scheduling_choices: vec![1, 0, 2],
        nondet_choices: vec![false, true],
    };
    let path = std::env::temp_dir().join("test_schedule.txt");
    schedule.save(&path).unwrap();
    let loaded = party::checker::trace::Schedule::load(&path).unwrap();
    assert_eq!(schedule.scheduling_choices, loaded.scheduling_choices);
    assert_eq!(schedule.nondet_choices, loaded.nondet_choices);
    std::fs::remove_file(&path).ok();
}

#[test]
fn replay_reproduces_assertion_failure() {
    let program = compile_programs(r#"
        machine Main {
            start state S {
                entry {
                    if ($) {
                        assert false, "found the bug";
                    }
                }
            }
        }
    "#);

    // Run until we find the bug, capturing the schedule
    let mut found_schedule = None;
    for _ in 0..100 {
        let mut rt = party::checker::runtime::Runtime::new(&program.programs);
        let result = rt.run();
        if result.is_err() {
            found_schedule = Some(rt.get_schedule());
            break;
        }
    }
    let schedule = found_schedule.expect("should find the bug in 100 tries");

    // Replay — must reproduce the same error
    let mut rt = party::checker::runtime::Runtime::new(&program.programs);
    rt.set_schedule(schedule);
    let result = rt.run();
    assert!(result.is_err(), "replay should reproduce the bug");
    assert!(result.unwrap_err().message.contains("found the bug"));
}

#[test]
fn replay_reproduces_same_trace() {
    let program = compile_programs(r#"
        event E;
        machine Main {
            var x: int;
            start state S {
                entry {
                    if ($) { x = 1; } else { x = 2; }
                    send this, E;
                }
                on E do { assert x == 1 || x == 2; }
            }
        }
    "#);

    // Run once, get schedule and trace
    let mut rt = party::checker::runtime::Runtime::new(&program.programs);
    let _ = rt.run();
    let schedule = rt.get_schedule();
    let trace1 = rt.get_trace();

    // Replay with same schedule
    let mut rt2 = party::checker::runtime::Runtime::new(&program.programs);
    rt2.set_schedule(schedule);
    let _ = rt2.run();
    let trace2 = rt2.get_trace();

    // Traces should match
    assert_eq!(trace1.len(), trace2.len(),
        "replay trace length should match: {} vs {}", trace1.len(), trace2.len());
    for (i, (a, b)) in trace1.iter().zip(trace2.iter()).enumerate() {
        assert_eq!(a, b, "trace mismatch at step {i}");
    }
}

#[test]
fn replay_deterministic_across_runs() {
    let program = compile_programs(r#"
        event Ping: machine;
        event Pong;
        machine Main {
            var w: machine;
            start state S {
                entry {
                    w = new Worker(this);
                    send w, Ping, this;
                }
                on Pong do { }
            }
        }
        machine Worker {
            start state S {
                entry (payload: machine) { }
                on Ping do (payload: machine) { send payload, Pong; }
            }
        }
    "#);

    // Get a schedule
    let mut rt = party::checker::runtime::Runtime::new(&program.programs);
    let _ = rt.run();
    let schedule = rt.get_schedule();

    // Replay 5 times — all should produce identical traces
    let mut traces = Vec::new();
    for _ in 0..5 {
        let mut rt = party::checker::runtime::Runtime::new(&program.programs);
        rt.set_schedule(schedule.clone());
        let _ = rt.run();
        traces.push(rt.get_trace());
    }

    for i in 1..traces.len() {
        assert_eq!(traces[0], traces[i], "replay {i} should match replay 0");
    }
}
