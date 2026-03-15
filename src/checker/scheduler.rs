//! Scheduling strategies for systematic exploration of P programs.
//! Implements a DFS strategy that mirrors PChecker's exhaustive exploration.

use log::{debug, trace};

/// A choice at a given depth level.
#[derive(Debug, Clone)]
struct Choice<T: Clone> {
    value: T,
    is_done: bool,
}

/// DFS scheduling strategy with backtracking.
/// Maintains three stacks for exhaustive exploration:
/// 1. Schedule stack — which machine to step next
/// 2. Boolean nondet stack — outcomes of $ and $$ choices
/// 3. Integer nondet stack — outcomes of choose(n) choices
pub struct DfsScheduler {
    /// Scheduling choices at each depth level.
    /// Each level contains the enabled machine IDs at that scheduling point.
    schedule_stack: Vec<Vec<Choice<usize>>>,
    /// Boolean nondeterministic choices at each depth.
    bool_nondet_stack: Vec<Vec<Choice<bool>>>,
    /// Integer nondeterministic choices at each depth.
    int_nondet_stack: Vec<Vec<Choice<i64>>>,
    /// Current depth in the schedule stack.
    sch_index: usize,
    /// Current depth in the nondeterministic stacks.
    nondet_index: usize,
    /// Total steps taken in current iteration.
    scheduled_steps: usize,
    /// Maximum steps per iteration.
    max_steps: usize,
    /// Whether we've exhausted the search space.
    exhausted: bool,
}

impl DfsScheduler {
    pub fn new(max_steps: usize) -> Self {
        Self {
            schedule_stack: Vec::new(),
            bool_nondet_stack: Vec::new(),
            int_nondet_stack: Vec::new(),
            sch_index: 0,
            nondet_index: 0,
            scheduled_steps: 0,
            max_steps,
            exhausted: false,
        }
    }

    /// Whether the DFS has exhausted all schedules.
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Choose the next machine to schedule from the enabled set.
    /// Returns None if all choices at current depth have been explored (backtrack signal).
    pub fn get_next_operation(&mut self, enabled: &[usize]) -> Option<usize> {
        if enabled.is_empty() {
            return None;
        }

        self.scheduled_steps += 1;
        if self.scheduled_steps > self.max_steps {
            return None;
        }

        if self.sch_index < self.schedule_stack.len() {
            // Revisiting a depth: use pre-existing choices
            // Find first not-yet-done choice that is also enabled
            let pos = self.schedule_stack[self.sch_index].iter()
                .position(|c| !c.is_done && enabled.contains(&c.value));
            if let Some(pos) = pos {
                self.schedule_stack[self.sch_index][pos].is_done = true;
                let result = self.schedule_stack[self.sch_index][pos].value;
                self.sch_index += 1;
                trace!("DFS schedule depth={}: chose machine {} from {:?}", self.sch_index - 1, result, enabled);
                return Some(result);
            }
            // All done at this depth — just pick first enabled (replay)
            let result = enabled[0];
            self.sch_index += 1;
            Some(result)
        } else {
            // New depth: create choices for all enabled operations
            let choices: Vec<Choice<usize>> = enabled.iter().map(|&id| Choice { value: id, is_done: false }).collect();
            self.schedule_stack.push(choices);
            // Pick the first one
            self.schedule_stack[self.sch_index][0].is_done = true;
            let result = enabled[0];
            self.sch_index += 1;
            trace!("DFS schedule depth={}: new choices {:?}, chose {}", self.sch_index - 1, enabled, result);
            Some(result)
        }
    }

    /// Get next boolean nondeterministic choice.
    pub fn get_next_boolean_choice(&mut self) -> Option<bool> {
        self.scheduled_steps += 1;
        if self.scheduled_steps > self.max_steps {
            return None;
        }

        if self.nondet_index < self.bool_nondet_stack.len() {
            let pos = self.bool_nondet_stack[self.nondet_index].iter()
                .position(|c| !c.is_done);
            if let Some(pos) = pos {
                self.bool_nondet_stack[self.nondet_index][pos].is_done = true;
                let result = self.bool_nondet_stack[self.nondet_index][pos].value;
                self.nondet_index += 1;
                trace!("DFS bool nondet depth={}: chose {}", self.nondet_index - 1, result);
                return Some(result);
            }
            // All done — pick first (replay)
            let result = self.bool_nondet_stack[self.nondet_index][0].value;
            self.nondet_index += 1;
            Some(result)
        } else {
            // New depth: [false, true]
            let choices = vec![
                Choice { value: false, is_done: false },
                Choice { value: true, is_done: false },
            ];
            self.bool_nondet_stack.push(choices);
            self.bool_nondet_stack[self.nondet_index][0].is_done = true;
            self.nondet_index += 1;
            trace!("DFS bool nondet depth={}: new, chose false", self.nondet_index - 1);
            Some(false)
        }
    }

    /// Get next integer nondeterministic choice in range [0, max_value).
    pub fn get_next_integer_choice(&mut self, max_value: i64) -> Option<i64> {
        self.scheduled_steps += 1;
        if self.scheduled_steps > self.max_steps {
            return None;
        }

        if self.nondet_index < self.int_nondet_stack.len() {
            let choices = &mut self.int_nondet_stack[self.nondet_index];
            if let Some(pos) = choices.iter().position(|c| !c.is_done) {
                choices[pos].is_done = true;
                let result = choices[pos].value;
                self.nondet_index += 1;
                return Some(result);
            }
            let result = choices[0].value;
            self.nondet_index += 1;
            Some(result)
        } else {
            let choices: Vec<Choice<i64>> = (0..max_value).map(|v| Choice { value: v, is_done: false }).collect();
            self.int_nondet_stack.push(choices);
            if !self.int_nondet_stack[self.nondet_index].is_empty() {
                self.int_nondet_stack[self.nondet_index][0].is_done = true;
            }
            self.nondet_index += 1;
            Some(0)
        }
    }

    /// Prepare for the next iteration. Returns false if all schedules have been explored.
    pub fn prepare_for_next_iteration(&mut self) -> bool {
        // Check if fully explored
        if self.schedule_stack.is_empty() {
            self.exhausted = true;
            return false;
        }

        // Reset indices
        self.sch_index = 0;
        self.nondet_index = 0;
        self.scheduled_steps = 0;

        // Phase 1: Backtrack bool nondet stack
        let bool_backtracked = self.backtrack_stack(&mut self.bool_nondet_stack.clone(), "bool_nondet");
        if bool_backtracked {
            self.bool_nondet_stack = self.bool_nondet_stack.clone();
        }

        // Phase 2: Backtrack int nondet stack
        let _int_backtracked = self.backtrack_stack_int();

        // Phase 3: If both nondet stacks are fully explored, backtrack schedule stack
        let bool_empty = self.bool_nondet_stack.is_empty() || self.all_done_bool();
        let int_empty = self.int_nondet_stack.is_empty() || self.all_done_int();

        if bool_empty && int_empty {
            // Clear nondet stacks
            self.bool_nondet_stack.clear();
            self.int_nondet_stack.clear();

            // Backtrack schedule stack
            if !self.backtrack_schedule() {
                self.exhausted = true;
                return false;
            }
        } else {
            // Nondet stacks still have work — reset last schedule choice
            // so it can be replayed with different nondet outcomes
            if let Some(last_level) = self.schedule_stack.last_mut() {
                if let Some(last_done) = last_level.iter_mut().rev().find(|c| c.is_done) {
                    last_done.is_done = false;
                }
            }
        }

        debug!("DFS prepared next iteration: sch_stack_depth={}, bool_stack_depth={}, int_stack_depth={}",
            self.schedule_stack.len(), self.bool_nondet_stack.len(), self.int_nondet_stack.len());

        true
    }

    fn backtrack_stack(&mut self, _stack: &[Vec<Choice<bool>>], _name: &str) -> bool {
        // Find deepest level with unexplored choices
        let stack = &mut self.bool_nondet_stack;
        if stack.is_empty() {
            return false;
        }

        // Walk from back to find a level with unexplored choices
        let mut backtrack_to = None;
        for i in (0..stack.len()).rev() {
            if stack[i].iter().any(|c| !c.is_done) {
                backtrack_to = Some(i);
                break;
            }
        }

        if let Some(level) = backtrack_to {
            // Truncate stack to this level + 1
            stack.truncate(level + 1);
            true
        } else {
            false
        }
    }

    fn backtrack_stack_int(&mut self) -> bool {
        let stack = &mut self.int_nondet_stack;
        if stack.is_empty() {
            return false;
        }

        let mut backtrack_to = None;
        for i in (0..stack.len()).rev() {
            if stack[i].iter().any(|c| !c.is_done) {
                backtrack_to = Some(i);
                break;
            }
        }

        if let Some(level) = backtrack_to {
            stack.truncate(level + 1);
            true
        } else {
            false
        }
    }

    fn backtrack_schedule(&mut self) -> bool {
        let stack = &mut self.schedule_stack;
        if stack.is_empty() {
            return false;
        }

        // Find deepest level with unexplored choices
        let mut backtrack_to = None;
        for i in (0..stack.len()).rev() {
            if stack[i].iter().any(|c| !c.is_done) {
                backtrack_to = Some(i);
                break;
            }
        }

        if let Some(level) = backtrack_to {
            stack.truncate(level + 1);
            true
        } else {
            false
        }
    }

    fn all_done_bool(&self) -> bool {
        self.bool_nondet_stack.iter().all(|level| level.iter().all(|c| c.is_done))
    }

    fn all_done_int(&self) -> bool {
        self.int_nondet_stack.iter().all(|level| level.iter().all(|c| c.is_done))
    }
}
