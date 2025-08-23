// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

pub struct Counter {
    pub value: Arc<Mutex<usize>>,
    compare: usize,
}

impl Counter {
    pub fn new(compare: usize) -> Self {
        Counter {
            value: Arc::new(Mutex::new(0)),
            compare,
        }
    }

    pub fn increase_check_reset(&self) -> bool {
        let mut val = self.value.lock().unwrap();
        *val += 1;
        if *val == self.compare {
            *val = 0;
            return true;
        }
        false
    }

    pub fn update_compare(&mut self, new_compare: usize) {
        let lock = self.value.lock().unwrap();
        self.compare = new_compare;
        drop(lock);
    }
}
