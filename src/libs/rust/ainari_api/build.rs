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

use chrono::Local;
use std::process::Command;

fn main() {
    let commit_hash = run_cmd(&["rev-parse", "--short", "HEAD"]);

    // try to get tag-name of the current commit.
    // if the commit is not tagged, use the name of the branch.
    let tag = run_cmd(&["tag", "--points-at", "HEAD"]);
    let version_string = if !tag.is_empty() {
        tag
    } else {
        run_cmd(&["rev-parse", "--abbrev-ref", "HEAD"])
    };

    // Fallback if everything fails
    let final_version = if version_string.trim().is_empty() {
        "unknown".to_string()
    } else {
        version_string
    };

    // get the timestamp, when this binary was compiled
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    println!("cargo:rustc-env=GIT_VERSION={}", final_version);
    println!("cargo:rustc-env=COMMIT_HASH={}", commit_hash);
    println!("cargo:rustc-env=COMPILE_TIMESTAMP={}", timestamp);
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");
    println!("cargo:rerun-if-changed=build.rs");
}

fn run_cmd(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}
