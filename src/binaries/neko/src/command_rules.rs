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

use regex::Regex;
use std::sync::LazyLock;

use crate::regex_rules::*;

// Defines how an argument position is evaluated
pub enum ArgMatcher {
    Exact(&'static str),
    Regex(&'static Regex),
}

// Defines a command allowlist rule
pub struct CommandRule {
    pub command: &'static str,
    pub args: Vec<ArgMatcher>,
}

impl CommandRule {
    pub fn matches(&self, cmd: &str, args: &[String]) -> bool {
        if self.command != cmd || self.args.len() != args.len() {
            return false;
        }

        for (matcher, arg) in self.args.iter().zip(args.iter()) {
            let valid = match matcher {
                ArgMatcher::Exact(expected) => *expected == arg,
                ArgMatcher::Regex(re) => re.is_match(arg),
            };

            if !valid {
                return false;
            }
        }
        true
    }
}

pub static COMMAND_RULES: LazyLock<Vec<CommandRule>> = LazyLock::new(|| {
    vec![
        // 1. ip tuntap add mode tap <name>
        CommandRule {
            command: "ip",
            args: vec![
                ArgMatcher::Exact("tuntap"),
                ArgMatcher::Exact("add"),
                ArgMatcher::Exact("mode"),
                ArgMatcher::Exact("tap"),
                ArgMatcher::Regex(&IFACE_NAME_REGEX),
            ],
        },
        // 2. ip link set <name> up
        CommandRule {
            command: "ip",
            args: vec![
                ArgMatcher::Exact("link"),
                ArgMatcher::Exact("set"),
                ArgMatcher::Regex(&IFACE_NAME_REGEX),
                ArgMatcher::Exact("up"),
            ],
        },
        // 3. ethtool -K <name> tx off rx off
        CommandRule {
            command: "ethtool",
            args: vec![
                ArgMatcher::Exact("-K"),
                ArgMatcher::Regex(&IFACE_NAME_REGEX),
                ArgMatcher::Exact("tx"),
                ArgMatcher::Exact("off"),
                ArgMatcher::Exact("rx"),
                ArgMatcher::Exact("off"),
            ],
        },
        // 4. ip addr add <ip_cidr> dev <name>
        CommandRule {
            command: "ip",
            args: vec![
                ArgMatcher::Exact("addr"),
                ArgMatcher::Exact("add"),
                ArgMatcher::Regex(&IP_CIDR_REGEX),
                ArgMatcher::Exact("dev"),
                ArgMatcher::Regex(&IFACE_NAME_REGEX),
            ],
        },
    ]
});
