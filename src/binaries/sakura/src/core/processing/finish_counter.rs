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

// ==================================================================================================

#[derive(Default, Debug)]
pub struct FinishCounter {
    pub input_compare: usize,
    pub output_compare: usize,
    task_compare: usize,
    counter: usize,

    expected_cycle_number: u64,
    already_finished: bool,

    pub task: Option<Arc<Mutex<Task>>>,
}

impl FinishCounter {
    pub fn reset(&mut self, task_compare: usize, expected_cycle_number: u64) {
        self.expected_cycle_number = expected_cycle_number;
        self.counter = 0;
        self.already_finished = false;
        self.task_compare = task_compare;
        self.task = None;
    }

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

    pub fn is_finished(&self) -> bool {
        if self.counter >= self.task_compare {
            return true;
        }

        false
    }

    pub fn get_expected_cycle_number(&self) -> u64 {
        self.expected_cycle_number
    }
}
