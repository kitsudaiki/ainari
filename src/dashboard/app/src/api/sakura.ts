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

// Endpoints of the sakura, mirroring `src/binaries/sakura/src/api/routes/v1alpha.rs`.
//
// The sakura is never addressed directly, but always through the torii, which opens
// one port per virtual-machine. Every function here therefore takes the torii-port
// of the virtual-machine it belongs to.

import { sakuraClient } from "./client";
import type {
    TaskBasicResp,
    TaskSnapshotRestoreReq,
    TaskSnapshotSaveReq,
    TaskResp,
    VirtualMachineCreateTaskReq,
} from "./types";

//=============================================================================
// task-paths
//=============================================================================

// The route-table of the sakura registers the task-endpoints flat below `/v1alpha/task`,
// while their handlers (`list_task`, `get_task`, `abort_task`) still extract the
// virtual-machine-uuid from the path and expect the nested form
// `/v1alpha/virtual_machine/{virtual_machine_uuid}/task/...`. Both sides have to agree
// again before the task-endpoints can work. Until then this flag selects the variant
// the dashboard uses, so that only one line has to change.
//
// `false` -> flat paths, as registered in `routes/v1alpha.rs`
// `true`  -> nested paths, as expected by the handlers and used by the go-sdk
const USE_NESTED_TASK_PATHS = false;

/** Builds the path of the task-list of a virtual-machine. */
function taskListPath(virtualMachineUuid: string): string {
    return USE_NESTED_TASK_PATHS
        ? `/v1alpha/virtual_machine/${virtualMachineUuid}/task`
        : "/v1alpha/task";
}

/** Builds the path of a single task of a virtual-machine. */
function taskPath(virtualMachineUuid: string, taskUuid: string): string {
    return USE_NESTED_TASK_PATHS
        ? `/v1alpha/virtual_machine/${virtualMachineUuid}/task/${taskUuid}`
        : `/v1alpha/task/${taskUuid}`;
}

//=============================================================================
// task
//=============================================================================

/**
 * `GET /v1alpha/task?resource_uuid=...` - all tasks of the virtual-machine. The sakura behind
 * the torii-port holds the tasks of all virtual-machines of its host, so they are filtered by
 * the uuid of the virtual-machine.
 */
export async function listTasks(
    toriiPort: number,
    virtualMachineUuid: string,
): Promise<TaskBasicResp[]> {
    const resp = await sakuraClient(toriiPort).get(taskListPath(virtualMachineUuid), {
        params: { resource_uuid: virtualMachineUuid },
    });
    return resp.data.tasks;
}

/** `GET /v1alpha/task/{task_uuid}` */
export async function getTask(
    toriiPort: number,
    virtualMachineUuid: string,
    taskUuid: string,
): Promise<TaskResp> {
    const resp = await sakuraClient(toriiPort).get(
        taskPath(virtualMachineUuid, taskUuid),
    );
    return resp.data;
}

/** `PUT /v1alpha/task/{task_uuid}/abort` */
export async function abortTask(
    toriiPort: number,
    virtualMachineUuid: string,
    taskUuid: string,
): Promise<TaskResp> {
    const resp = await sakuraClient(toriiPort).put(
        `${taskPath(virtualMachineUuid, taskUuid)}/abort`,
        {},
    );
    return resp.data;
}

//=============================================================================
// virtual-machine
//=============================================================================

/**
 * `POST /v1alpha/virtual_machine/{virtual_machine_uuid}`
 *
 * Creates the task, which installs the image and the public-key on an already
 * reserved virtual-machine and boots it. The reservation itself is done beforehand
 * on the hanami, see `hanami.reserveVirtualMachine`.
 */
export async function createVirtualMachine(
    toriiPort: number,
    body: VirtualMachineCreateTaskReq,
): Promise<TaskResp> {
    const resp = await sakuraClient(toriiPort).post(
        `/v1alpha/virtual_machine/${body.vm_uuid}`,
        body,
    );
    return resp.data;
}

/** `POST /v1alpha/virtual_machine/{virtual_machine_uuid}/snapshot_save` */
export async function createSnapshotSaveTask(
    toriiPort: number,
    virtualMachineUuid: string,
    body: TaskSnapshotSaveReq,
): Promise<TaskResp> {
    const resp = await sakuraClient(toriiPort).post(
        `/v1alpha/virtual_machine/${virtualMachineUuid}/snapshot_save`,
        body,
    );
    return resp.data;
}

/** `POST /v1alpha/virtual_machine/{virtual_machine_uuid}/snapshot_restore` */
export async function createSnapshotRestoreTask(
    toriiPort: number,
    virtualMachineUuid: string,
    body: TaskSnapshotRestoreReq,
): Promise<TaskResp> {
    const resp = await sakuraClient(toriiPort).post(
        `/v1alpha/virtual_machine/${virtualMachineUuid}/snapshot_restore`,
        body,
    );
    return resp.data;
}
