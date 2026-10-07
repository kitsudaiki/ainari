# Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""
All suites of the local setup in the order, in which they run. A new suite is a module with a
`suite = Suite(...)`, which is added to this list.
"""

from . import (auth, cleanup, floating_ips, host_isolation, hosts, network_filters, networking,
               power, projects, proxies, resources, snapshots, ssh, tasks, users, virtual_machines)

ALL_SUITES = [
    auth.suite,
    hosts.suite,
    resources.suite,
    host_isolation.suite,
    virtual_machines.suite,
    proxies.suite,
    floating_ips.suite,
    ssh.suite,
    networking.suite,
    network_filters.suite,
    users.suite,
    projects.suite,
    power.suite,
    snapshots.suite,
    tasks.suite,
    cleanup.suite,
]
