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

/// A thread-safe counter that can be used to track and compare values.
///
/// This struct provides a counter that can be incremented across threads and
/// checked against a comparison value. When the counter reaches the comparison
/// value, it can be automatically reset.
pub struct Counter {
    /// The current value of the counter, protected by a mutex for thread safety.
    ///
    /// This value can be incremented and checked by multiple threads simultaneously.
    pub value: Arc<Mutex<usize>>,

    /// The comparison value that the counter will be checked against.
    ///
    /// When the counter's value matches this value, it will be automatically reset.
    compare: usize,
}

impl Counter {
    /// Creates a new `Counter` with the specified comparison value.
    ///
    /// The counter starts with a value of 0.
    ///
    /// # Arguments
    ///
    /// * `compare` - The value to compare against when checking the counter.
    pub fn new(compare: usize) -> Self {
        Counter {
            value: Arc::new(Mutex::new(0)), // Initialize counter value to 0
            compare,                        // Set the comparison value
        }
    }

    /// Increments the counter and checks if it matches the comparison value.
    ///
    /// If the counter's value matches the comparison value, it is reset to 0
    /// and this function returns `true`. Otherwise, it returns `false`.
    ///
    /// # Returns
    ///
    /// * `true` if the counter was reset due to matching the comparison value.
    /// * `false` otherwise.
    pub fn increase_check_reset(&self) -> bool {
        // Lock the mutex to safely access the counter value
        let mut val = self.value.lock().expect("mutex poisoned");

        // Increment the counter value
        *val += 1;

        // Check if the counter value matches the comparison value
        if *val == self.compare {
            // Reset the counter if it matches the comparison value
            *val = 0;
            return true;
        }
        false
    }

    /// Updates the comparison value of the counter.
    ///
    /// # Arguments
    ///
    /// * `new_compare` - The new value to compare against when checking the counter.
    pub fn update_compare(&mut self, new_compare: usize) {
        // Lock the mutex to ensure thread safety while updating the comparison value
        let lock = self.value.lock().expect("mutex poisoned");

        // Update the comparison value
        self.compare = new_compare;

        // Explicitly drop the lock to allow other threads to access the counter
        drop(lock);
    }
}
