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

use autocxx_build::Builder;

fn main() {
    Builder::new("src/lib.rs", &["src"])
        .extra_clang_args(&["-std=c++17"])
        .build().unwrap()
        .file("src/hanami_root.h")     
        .std("c++17")
        .compile("shapes-rs");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=src/hanami_root.h");
    // the release-version of the library has to be used to avoid linking-problems
    // with the ASAN-dependencies in the debug-version
    println!("cargo:rustc-link-search=/tmp/hanami_core/release");
    println!("cargo:rustc-link-lib=hanami_core");
}