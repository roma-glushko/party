# Party

**P** l**A**nguage in **R**us**T** — a formal verification toolchain for distributed systems.

Party is a from-scratch Rust implementation of the [P language](https://p-org.github.io/P/) compiler and model checker. 
P lets you model distributed systems as communicating state machines and automatically verify safety and liveness properties.

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

Hello world example:

```p
event Ping;
event Pong;

machine Pinger {
  var ponger: machine;

  start state Init {
    entry (p: machine) {
      ponger = p;
      send ponger, Ping;
    }

    on Pong do {
      print "Pinger received Pong!";
    }
  }
}

machine Ponger {
  start state Wait {
    on Ping do {
      print "Ponger received Ping!";
      send sender, Pong;
    }
  }
}

machine Main {
  start state Start {
    entry {
      var ponger: machine;
      var pinger: machine;

      ponger = new Ponger();
      pinger = new Pinger(ponger);
    }
  }
}
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

- **Catch bugs before production** — Finds race conditions, deadlocks, and protocol violations that tests miss
- **See exactly what went wrong** — Counterexample traces show the step-by-step interleaving that triggered the bug
- **Reproduce any bug on demand** — Schedules are saved to `.prun` files for deterministic replay, every time
- **Fast feedback loop** — Lint catches type errors and spec violations instantly, before model checking
- **Clean code, enforced** — Auto-formatter with `--check` mode for CI, consistent style across your team
- **Safety and liveness** — Verify both "bad things never happen" and "good things eventually happen"

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
