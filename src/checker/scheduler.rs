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
    schedule_stack: Vec<Vec<Choice<usize>>>,
    bool_nondet_stack: Vec<Vec<Choice<bool>>>,
    int_nondet_stack: Vec<Vec<Choice<i64>>>,
    sch_index: usize,
    bool_index: usize,
    int_index: usize,
    scheduled_steps: usize,
    max_steps: usize,
    exhausted: bool,
}

impl DfsScheduler {
    pub fn new(max_steps: usize) -> Self {
        Self {
            schedule_stack: Vec::new(),
            bool_nondet_stack: Vec::new(),
            int_nondet_stack: Vec::new(),
            sch_index: 0,
            bool_index: 0,
            int_index: 0,
            scheduled_steps: 0,
            max_steps,
            exhausted: false,
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    pub fn get_next_operation(&mut self, enabled: &[usize]) -> Option<usize> {
        if enabled.is_empty() {
            return None;
        }

        self.scheduled_steps += 1;
        if self.scheduled_steps > self.max_steps {
            return None;
        }

        if self.sch_index < self.schedule_stack.len() {
            let pos = self.schedule_stack[self.sch_index].iter()
                .position(|c| !c.is_done && enabled.contains(&c.value));
            if let Some(pos) = pos {
                self.schedule_stack[self.sch_index][pos].is_done = true;
                let result = self.schedule_stack[self.sch_index][pos].value;
                self.sch_index += 1;
                trace!("DFS schedule depth={}: chose machine {} from {:?}", self.sch_index - 1, result, enabled);
                return Some(result);
            }
            let result = enabled[0];
            self.sch_index += 1;
            Some(result)
        } else {
            let choices: Vec<Choice<usize>> = enabled.iter().map(|&id| Choice { value: id, is_done: false }).collect();
            self.schedule_stack.push(choices);
            self.schedule_stack[self.sch_index][0].is_done = true;
            let result = enabled[0];
            self.sch_index += 1;
            trace!("DFS schedule depth={}: new choices {:?}, chose {}", self.sch_index - 1, enabled, result);
            Some(result)
        }
    }

    pub fn get_next_boolean_choice(&mut self) -> Option<bool> {
        self.scheduled_steps += 1;
        if self.scheduled_steps > self.max_steps {
            return None;
        }

        if self.bool_index < self.bool_nondet_stack.len() {
            let pos = self.bool_nondet_stack[self.bool_index].iter()
                .position(|c| !c.is_done);
            if let Some(pos) = pos {
                self.bool_nondet_stack[self.bool_index][pos].is_done = true;
                let result = self.bool_nondet_stack[self.bool_index][pos].value;
                self.bool_index += 1;
                trace!("DFS bool nondet depth={}: chose {}", self.bool_index - 1, result);
                return Some(result);
            }
            let result = self.bool_nondet_stack[self.bool_index][0].value;
            self.bool_index += 1;
            Some(result)
        } else {
            let choices = vec![
                Choice { value: false, is_done: false },
                Choice { value: true, is_done: false },
            ];
            self.bool_nondet_stack.push(choices);
            self.bool_nondet_stack[self.bool_index][0].is_done = true;
            self.bool_index += 1;
            trace!("DFS bool nondet depth={}: new, chose false", self.bool_index - 1);
            Some(false)
        }
    }

    pub fn get_next_integer_choice(&mut self, max_value: i64) -> Option<i64> {
        self.scheduled_steps += 1;
        if self.scheduled_steps > self.max_steps {
            return None;
        }

        if self.int_index < self.int_nondet_stack.len() {
            let choices = &mut self.int_nondet_stack[self.int_index];
            if let Some(pos) = choices.iter().position(|c| !c.is_done && c.value < max_value) {
                choices[pos].is_done = true;
                let result = choices[pos].value;
                self.int_index += 1;
                return Some(result);
            }
            // All valid choices done — pick 0 (safe default within range)
            self.int_index += 1;
            Some(0)
        } else {
            let choices: Vec<Choice<i64>> = (0..max_value).map(|v| Choice { value: v, is_done: false }).collect();
            self.int_nondet_stack.push(choices);
            if !self.int_nondet_stack[self.int_index].is_empty() {
                self.int_nondet_stack[self.int_index][0].is_done = true;
            }
            self.int_index += 1;
            Some(0)
        }
    }

    pub fn prepare_for_next_iteration(&mut self) -> bool {
        if self.schedule_stack.is_empty() {
            self.exhausted = true;
            return false;
        }

        self.sch_index = 0;
        self.bool_index = 0;
        self.int_index = 0;
        self.scheduled_steps = 0;

        // Phase 1: Backtrack bool nondet stack
        let bool_backtracked = Self::backtrack_generic(&mut self.bool_nondet_stack);

        // Phase 2: Backtrack int nondet stack
        let int_backtracked = Self::backtrack_generic(&mut self.int_nondet_stack);

        // Phase 3: If both nondet stacks are fully explored, backtrack schedule stack
        let bool_empty = self.bool_nondet_stack.is_empty()
            || self.bool_nondet_stack.iter().all(|level| level.iter().all(|c| c.is_done));
        let int_empty = self.int_nondet_stack.is_empty()
            || self.int_nondet_stack.iter().all(|level| level.iter().all(|c| c.is_done));

        if bool_empty && int_empty {
            self.bool_nondet_stack.clear();
            self.int_nondet_stack.clear();

            if !Self::backtrack_generic(&mut self.schedule_stack) {
                self.exhausted = true;
                return false;
            }
        } else {
            // Nondet stacks still have work — reset last schedule choice
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

    fn backtrack_generic<T: Clone>(stack: &mut Vec<Vec<Choice<T>>>) -> bool {
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
}
