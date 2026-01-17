// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::{Arc, Mutex};

use super::tasks::Task;

/// A counter that tracks the completion of tasks across multiple cycles.
/// This struct is used to monitor when a certain number of tasks have completed
/// within a specific cycle, and can trigger actions when the completion criteria are met.
#[derive(Default, Debug)]
pub struct FinishCounter {
    /// Number of input tasks that have been compared in the current cycle.
    pub input_compare: usize,

    /// Number of output tasks that have been compared in the current cycle.
    pub output_compare: usize,

    /// The threshold number of task comparisons needed to consider the cycle finished.
    task_compare: usize,

    /// Current count of completed tasks in the current cycle.
    counter: usize,

    /// The cycle number we're expecting to complete.
    expected_cycle_number: u64,

    /// Flag indicating whether this cycle has already been marked as finished.
    already_finished: bool,

    /// Optional reference to the task that this counter is associated with.
    pub task: Option<Arc<Mutex<Task>>>,
}

impl FinishCounter {
    /// Resets the counter for a new cycle.
    ///
    /// # Arguments
    ///
    /// * `task_compare` - The number of task comparisons needed to finish the cycle.
    /// * `expected_cycle_number` - The cycle number we're expecting to complete.
    ///
    /// This method initializes the counter to zero, resets the finished flag,
    /// and sets the task comparison threshold for the new cycle.
    pub fn reset(&mut self, task_compare: usize, expected_cycle_number: u64) {
        self.expected_cycle_number = expected_cycle_number;
        self.counter = 0;
        self.already_finished = false;
        self.task_compare = task_compare;
        self.task = None;
    }

    /// Updates the counter for the current cycle.
    ///
    /// # Arguments
    ///
    /// * `cycle_number` - The current cycle number.
    ///
    /// This method increments the counter if we're in the expected cycle.
    /// If the counter reaches the comparison threshold and the cycle hasn't
    /// been finished already, it will mark the cycle as finished and trigger
    /// the task to finish its current cycle.
    pub fn update(&mut self, cycle_number: u64) {
        if cycle_number == self.expected_cycle_number {
            self.counter += 1;

            if self.is_finished() && !self.already_finished {
                self.already_finished = true;
                if let Some(task_mutex) = &self.task {
                    let mut task = task_mutex.lock().expect("mutex poisoned");
                    task.finish_cycle();
                    let next_cycle = self.expected_cycle_number + 1;
                    self.expected_cycle_number = next_cycle;
                    self.counter = 0;
                    self.already_finished = false;
                }
            }
        }
    }

    /// Checks if the current cycle has finished based on the comparison threshold.
    ///
    /// # Returns
    ///
    /// `true` if the counter has reached or exceeded the task comparison threshold,
    /// `false` otherwise.
    pub fn is_finished(&self) -> bool {
        if self.counter >= self.task_compare {
            return true;
        }

        false
    }

    /// Gets the expected cycle number that this counter is tracking.
    ///
    /// # Returns
    ///
    /// The current expected cycle number.
    pub fn get_expected_cycle_number(&self) -> u64 {
        self.expected_cycle_number
    }
}
