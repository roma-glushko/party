# Party

**P** l**A**nguage in **R**us**T** — a formal verification toolchain for distributed systems.

Party is a from-scratch Rust implementation of the [P language](https://p-org.github.io/P/) compiler and model checker. P lets you model distributed systems as communicating state machines and automatically verify safety and liveness properties.

## Quick start

```bash
cargo install --path .

# Check for errors
party lint myproject/

# Format source files
party format myproject/

# Verify correctness
party verify myproject/ -t TestSafety
```

## What it does

Party takes `.p` files describing state machines and specifications, then systematically explores all possible interleavings to find bugs:

```
$ party verify examples/raft -t TestRaft

.. Checking test case: TestRaft
Starting model checking ...
... Found a bug.
Error: Assertion failed: Safety violation: two leaders in term 44

=== Counterexample Trace (last 5 of 781 steps) ===
  <778> RaftNode#1(CandidateState) send eTimeout -> RaftTimer#5
  <779> RaftTimer#5(Active) send eTimeout -> RaftNode#1
  <780> RaftNode#3(CandidateState) -> LeaderState
  <781> SingleLeaderPerTerm#4(Monitoring) ERROR assert Safety violation
=== End Trace ===

Schedule saved to: examples/raft.prun
Replay with: party verify examples/raft --replay examples/raft.prun
```

When a bug is found, the schedule is saved to a `.prun` file for deterministic replay.

## Commands

```
party lint <path>                   Parse and type-check .p files
party format <path> [--check]       Auto-format .p files (--check for CI)
party verify <path> [-t test]       Run model checking
party verify <path> --replay f.prun Replay a saved schedule
```

## Features

- **Lexer & parser** — Full P language grammar, recursive descent with Pratt expression parsing
- **Type checker** — Subtyping, payload validation, purity analysis, 137/137 static error tests
- **Model checker** — DFS + random scheduling, nondeterministic exploration
- **Spec monitors** — Safety assertions, liveness via temperature-based hot/cold states
- **Formatter** — Consistent 2-space style, `--check` mode for CI
- **Counterexample traces** — Structured event log with step numbers
- **Schedule replay** — Save `.prun` files, reproduce bugs deterministically

## Test results

```
Correct programs    147/147   100%
Static errors       137/137   100%
Liveness            21/21     100%
Dynamic errors      104/107    97%
─────────────────────────────────
Total               409/412   99.3%
```

## Development

```bash
make help       # Show all targets
make lint       # Format check + clippy + P lint
make test       # Run all test suites
make build      # Debug build
make release    # Optimized build
```

## License

Same as the [P project](https://github.com/p-org/P) — MIT.
