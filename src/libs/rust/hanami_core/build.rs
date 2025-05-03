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

use std::env;
use std::path::PathBuf;

fn main() {
    // let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // compile C++ library via CMake
    let dst = cmake::Config::new("hanami_core_cpp")
        //.no_build_target(true) // <- this disables the 'install' target
        .build();

    // setup autocxx
    let include_path = "hanami_core_cpp";
    let mut b = autocxx_build::Builder::new("src/lib.rs", &[include_path])
        .extra_clang_args(&["-std=c++17"])
        .build()
        .unwrap();

    if env::var("PROFILE").unwrap() == "release" {
        b.flag_if_supported("-O3");
    }
    
    b.include(include_path)
     .flag_if_supported("-std=c++17");

    b.compile("autocxx-hanami_core_cpp");

    // link against C++ static lib
    println!("cargo:rustc-link-search=native={}/build", dst.display());
    println!("cargo:rustc-link-lib=static=hanami_core_cpp");
}

