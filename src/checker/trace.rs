//! Structured execution trace for P program model checking.
//! Records all runtime events for counterexample reporting.
//! Inspired by Go's runtime/trace — always-on, structured, low overhead.

use std::fmt;

/// A single trace event capturing a runtime action.
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub seq: usize,
    pub kind: TraceKind,
    pub machine: String,
    pub machine_id: usize,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraceKind {
    CreateMachine,
    StateTransition,
    SendEvent,
    RaiseEvent,
    AnnounceEvent,
    DequeueEvent,
    ReceiveEvent,
    GotoState,
    AssertionFailed,
    UnhandledEvent,
    Halted,
    LivenessViolation,
    Error,
}

impl fmt::Display for TraceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match &self.kind {
            TraceKind::CreateMachine => "create",
            TraceKind::StateTransition => "->",
            TraceKind::SendEvent => "send",
            TraceKind::RaiseEvent => "raise",
            TraceKind::AnnounceEvent => "announce",
            TraceKind::DequeueEvent => "dequeue",
            TraceKind::ReceiveEvent => "receive",
            TraceKind::GotoState => "goto",
            TraceKind::AssertionFailed => "ERROR assert",
            TraceKind::UnhandledEvent => "ERROR unhandled",
            TraceKind::Halted => "halt",
            TraceKind::LivenessViolation => "ERROR liveness",
            TraceKind::Error => "ERROR",
        };
        write!(
            f,
            "<{seq}> {machine}#{id}({state}) {kind} {detail}",
            seq = self.seq,
            machine = self.machine,
            id = self.machine_id,
            state = self.state,
            kind = kind_str,
            detail = self.detail,
        )
    }
}

/// In-flight trace recorder. Always active, captures all events.
#[derive(Debug, Clone)]
pub struct Tracer {
    events: Vec<TraceEvent>,
    seq: usize,
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            seq: 0,
        }
    }

    pub fn record(
        &mut self,
        kind: TraceKind,
        machine: &str,
        machine_id: usize,
        state: &str,
        detail: impl Into<String>,
    ) {
        self.seq += 1;
        self.events.push(TraceEvent {
            seq: self.seq,
            kind,
            machine: machine.to_string(),
            machine_id,
            state: state.to_string(),
            detail: detail.into(),
        });
    }

    /// Get all trace events.
    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    /// Format the full trace as strings.
    pub fn to_strings(&self) -> Vec<String> {
        self.events.iter().map(|e| e.to_string()).collect()
    }

    /// Print the trace to stderr (for counterexample reporting).
    pub fn print_trace(&self) {
        eprintln!("=== Execution Trace ({} steps) ===", self.events.len());
        for event in &self.events {
            eprintln!("  {event}");
        }
        eprintln!("=== End Trace ===");
    }

    /// Print only the last N events (for concise error reporting).
    pub fn print_tail(&self, n: usize) {
        let start = if self.events.len() > n { self.events.len() - n } else { 0 };
        eprintln!("=== Execution Trace (last {} of {} steps) ===", self.events.len() - start, self.events.len());
        if start > 0 {
            eprintln!("  ... ({start} earlier events omitted)");
        }
        for event in &self.events[start..] {
            eprintln!("  {event}");
        }
        eprintln!("=== End Trace ===");
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.seq = 0;
    }
}

/// A recorded schedule that can be saved and replayed.
/// Captures all scheduling and nondeterministic decisions.
#[derive(Debug, Clone)]
pub struct Schedule {
    /// Which machine index was chosen at each scheduling point.
    pub scheduling_choices: Vec<usize>,
    /// Nondeterministic boolean choices ($ and $$).
    pub nondet_choices: Vec<bool>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            scheduling_choices: Vec::new(),
            nondet_choices: Vec::new(),
        }
    }

    /// Serialize to a human-readable string.
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str("# PLang schedule replay file (.prun)\n");
        s.push_str("# Scheduling choices (machine index per step)\n");
        s.push_str("scheduling: ");
        s.push_str(&self.scheduling_choices.iter()
            .map(|c| c.to_string()).collect::<Vec<_>>().join(","));
        s.push('\n');
        s.push_str("# Nondeterministic boolean choices\n");
        s.push_str("nondet: ");
        s.push_str(&self.nondet_choices.iter()
            .map(|c| if *c { "T" } else { "F" }).collect::<Vec<_>>().join(","));
        s.push('\n');
        s
    }

    /// Parse from string.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut scheduling = Vec::new();
        let mut nondet = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() { continue; }

            if let Some(rest) = line.strip_prefix("scheduling:") {
                let rest = rest.trim();
                if !rest.is_empty() {
                    for part in rest.split(',') {
                        let part = part.trim();
                        if !part.is_empty() {
                            scheduling.push(part.parse::<usize>()
                                .map_err(|e| format!("bad scheduling choice: {e}"))?);
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("nondet:") {
                let rest = rest.trim();
                if !rest.is_empty() {
                    for part in rest.split(',') {
                        let part = part.trim();
                        match part {
                            "T" | "true" | "1" => nondet.push(true),
                            "F" | "false" | "0" => nondet.push(false),
                            "" => {}
                            _ => return Err(format!("bad nondet choice: {part}")),
                        }
                    }
                }
            }
        }

        Ok(Schedule {
            scheduling_choices: scheduling,
            nondet_choices: nondet,
        })
    }

    /// Save to a .prun file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, self.to_string())
            .map_err(|e| format!("failed to write {}: {e}", path.display()))
    }

    /// Load from a .prun file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        Self::parse(&content)
    }
}
