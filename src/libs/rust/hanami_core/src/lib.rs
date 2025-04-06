// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use autocxx::prelude::*;

autocxx::include_cpp! {
  #include "hanami_root.h"
  safety!(unsafe)
  generate!("HanamiRoot")
  generate!("create_root")
}

pub fn test() -> i32 {

    let square = ffi::create_root();
    println!("Area: {} units", square.area().0);
    println!("Perimeter: {} units", square.perimeter().0);
    return square.area().0;
}

pub fn add(left: u64, right: u64) -> u64 {
    return left + right;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(test(), 25);
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
