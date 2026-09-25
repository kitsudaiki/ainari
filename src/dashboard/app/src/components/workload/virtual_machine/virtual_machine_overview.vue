<!--
// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//         http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
-->

<template>
    <div class="card">
        <div class="card-label">Virtual Machines</div>
        <div class="card-content">
            <!-- Add button -->
            <button class="add-button" @click="openCreateModal">+</button>

            <table
                class="overview-table"
                v-if="virtualMachines.length > 0"
            >
                <thead>
                    <tr>
                        <th>UUID</th>
                        <th>Name</th>
                        <th class="address-column">Address</th>
                        <th class="resource-column">Cores</th>
                        <th class="resource-column">Memory</th>
                        <th class="resource-column">Disk</th>
                        <th class="state-column">State</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                    <tr
                        v-for="virtualMachine in virtualMachines"
                        :key="virtualMachine.uuid"
                    >
                        <td>{{ virtualMachine.uuid }}</td>
                        <td>{{ virtualMachine.name }}</td>
                        <td class="address-column">
                            {{ torii_base_address }}:{{
                                virtualMachine.proxy_port
                            }}
                        </td>
                        <td class="resource-column">
                            {{ formatResource(virtualMachine.number_of_cores) }}
                        </td>
                        <td class="resource-column">
                            {{ formatMemory(virtualMachine.memory_size) }}
                        </td>
                        <td class="resource-column">
                            {{ formatResource(virtualMachine.disk_size, "GiB") }}
                        </td>
                        <td class="state-column">
                            <span
                                class="state-light"
                                :class="
                                    'state-' +
                                    (vmStates[virtualMachine.uuid] ?? 'unknown')
                                "
                                :title="
                                    stateLabels[
                                        vmStates[virtualMachine.uuid] ??
                                            'unknown'
                                    ]
                                "
                            ></span>
                        </td>
                        <td>
                            <!-- Dropdown menu -->
                            <div
                                class="table-dropdown"
                                @click.stop="
                                    toggleDropdown(virtualMachine.uuid)
                                "
                            >
                                ⋮
                                <div
                                    v-if="openDropdown === virtualMachine.uuid"
                                    class="table-dropdown-menu"
                                >
                                    <button
                                        @click="openInfoModal(virtualMachine)"
                                    >
                                        Info
                                    </button>
                                    <button
                                        @click="
                                            switchToTasks(virtualMachine.uuid)
                                        "
                                    >
                                        Show tasks
                                    </button>
                                    <button
                                        @click="
                                            changePowerState(
                                                virtualMachine,
                                                'start',
                                            )
                                        "
                                    >
                                        Start
                                    </button>
                                    <button
                                        @click="
                                            changePowerState(
                                                virtualMachine,
                                                'stop',
                                            )
                                        "
                                    >
                                        Stop
                                    </button>
                                    <button
                                        @click="
                                            changePowerState(
                                                virtualMachine,
                                                'reboot',
                                            )
                                        "
                                    >
                                        Reboot
                                    </button>
                                    <button
                                        @click="
                                            openSnapshotSaveModal(virtualMachine)
                                        "
                                    >
                                        Save snapshot
                                    </button>
                                    <button
                                        @click="
                                            openSnapshotRestoreModal(
                                                virtualMachine,
                                            )
                                        "
                                    >
                                        Restore from snapshot
                                    </button>
                                    <button
                                        @click="openDeleteModal(virtualMachine)"
                                    >
                                        Delete
                                    </button>
                                </div>
                            </div>
                        </td>
                    </tr>
                </tbody>
            </table>

            <p v-else>No virtual machines found</p>
        </div>

        <VirtualMachineCreateModal
            v-if="showCreateModal"
            :icons="icons"
            @accept="acceptCreateModal"
            @reserved="fetchVirtualMachines"
            @cancel="cancelCreateModal"
        />

        <VirtualMachineInfoModal
            v-if="showInfoModal"
            :virtual_machine="virtualMachineToShow"
            :icons="icons"
            @cancel="cancelInfoModal"
        />

        <VirtualMachineDeleteModal
            v-if="showDeleteModal"
            :virtual_machine="virtualMachineToDelete"
            :icons="icons"
            @accept="acceptDeleteModal"
            @cancel="cancelDeleteModal"
        />

        <SnapshotSaveModal
            v-if="showSnapshotSaveModal"
            :virtual_machine_uuid="snapshotVirtualMachineUuid"
            :torii_port="snapshotToriiPort"
            :icons="icons"
            @accept="closeSnapshotModals"
            @cancel="closeSnapshotModals"
        />

        <SnapshotRestoreModal
            v-if="showSnapshotRestoreModal"
            :virtual_machine_uuid="snapshotVirtualMachineUuid"
            :torii_port="snapshotToriiPort"
            :icons="icons"
            @accept="closeSnapshotModals"
            @cancel="closeSnapshotModals"
        />
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, inject } from "vue";

import { getAuthContext } from "@/auth_context";
import { hanami, sakura } from "@/api";
import type { VirtualMachineBasicResp } from "@/api";
import VirtualMachineCreateModal from "./virtual_machine_create_modal.vue";
import VirtualMachineInfoModal from "./virtual_machine_info_modal.vue";
import VirtualMachineDeleteModal from "./virtual_machine_delete_modal.vue";
import SnapshotSaveModal from "./snapshot_save_modal.vue";
import SnapshotRestoreModal from "./snapshot_restore_modal.vue";
import { handleAxiosError } from "@/handleAxiosError";

const errorPopupMsg = ref<string>("");
const virtualMachines = ref<VirtualMachineBasicResp[]>([]);
const torii_base_address = ref<string | null>("");
const showCreateModal = ref(false);
const showInfoModal = ref(false);
const showDeleteModal = ref(false);
const openDropdown = ref<string | null>(null);
const virtualMachineToShow = ref<VirtualMachineBasicResp | null>(null);
const virtualMachineToDelete = ref<VirtualMachineBasicResp | null>(null);
const showSnapshotSaveModal = ref(false);
const showSnapshotRestoreModal = ref(false);
const snapshotVirtualMachineUuid = ref<string | null>(null);
// the torii-port of the virtual-machine, which is needed to reach its sakura
const snapshotToriiPort = ref<number>(0);

// State of each virtual machine, shown as traffic-light in the table
type VmState = "unknown" | "created" | "reserved" | "error";
const stateLabels: Record<VmState, string> = {
    unknown: "Unknown",
    created: "Created",
    reserved: "Reserved, not created yet",
    error: "Error",
};
const vmStates = ref<Record<string, VmState>>({});
let statePollTimer: ReturnType<typeof setInterval> | null = null;
let statePollRunning = false;
const icons = inject<{ acceptIcon: string; cancelIcon: string }>("icons")!;

const emit = defineEmits<{
    (e: "change-view", payload: { view: string; id: string }): void;
}>();

function switchToTasks(virtual_machine_uuid: string) {
    emit("change-view", { view: "WorkloadTask", id: virtual_machine_uuid });
}

//=============================================================================
// Resources
//=============================================================================
// Virtual machines of older versions have no stored resources, which is signaled by 0.
function formatResource(value: number, unit: string = ""): string {
    if (value <= 0) return "–";
    return unit ? `${value} ${unit}` : `${value}`;
}

/** The memory is provided in MiB, but bigger values are easier to read in GiB. */
function formatMemory(mib: number): string {
    if (mib < 1024) return formatResource(mib, "MiB");
    return `${Math.round((mib / 1024) * 10) / 10} GiB`;
}

async function fetchVirtualMachines() {
    try {
        torii_base_address.value = getAuthContext().torii_base_address;
        virtualMachines.value = await hanami.listVirtualMachines();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual machines",
        );
        return;
    }

    // drop states of virtual machines, which don't exist anymore
    const states: Record<string, VmState> = {};
    for (const virtualMachine of virtualMachines.value) {
        states[virtualMachine.uuid] =
            vmStates.value[virtualMachine.uuid] ?? "unknown";
    }
    vmStates.value = states;

    await Promise.all(
        virtualMachines.value.map((vm) => updateVirtualMachineState(vm.uuid)),
    );
}

//=============================================================================
// State of the virtual machines
//=============================================================================
async function fetchVirtualMachineState(uuid: string): Promise<VmState> {
    let virtualMachine;
    try {
        virtualMachine = await hanami.getVirtualMachine(uuid);
    } catch {
        return "error";
    }
    if (virtualMachine.is_created) {
        return "created";
    }

    // the virtual machine is not created yet, so check if the create-task failed
    try {
        const tasks = await sakura.listTasks(virtualMachine.torii_port, uuid);
        const failed = tasks.some(
            (task) =>
                task.task_type === "VirtualMachineCreate" &&
                (task.state === "Error" || task.state === "Aborted"),
        );
        return failed ? "error" : "reserved";
    } catch {
        // tasks not reachable (yet), so it counts as still in progress
        return "reserved";
    }
}

async function updateVirtualMachineState(uuid: string) {
    const state = await fetchVirtualMachineState(uuid);
    // the virtual machine could have been removed from the list in the meantime
    if (uuid in vmStates.value) {
        vmStates.value[uuid] = state;
    }
}

// checks every second all virtual machines, which are reserved but not created yet
async function pollReservedVirtualMachines() {
    // skip, if the previous check is still running
    if (statePollRunning) return;
    statePollRunning = true;
    try {
        const reserved = Object.keys(vmStates.value).filter(
            (uuid) => vmStates.value[uuid] === "reserved",
        );
        await Promise.all(reserved.map(updateVirtualMachineState));
    } finally {
        statePollRunning = false;
    }
}

//=============================================================================
// Dropdown in table
//=============================================================================
function toggleDropdown(uuid: string) {
    openDropdown.value = openDropdown.value === uuid ? null : uuid;
}

function handleClickOutside(event: MouseEvent) {
    const dropdowns = document.querySelectorAll(".table-dropdown");
    let clickedInside = false;
    dropdowns.forEach((dropdown) => {
        if (dropdown.contains(event.target as Node)) {
            clickedInside = true;
        }
    });
    if (!clickedInside) {
        openDropdown.value = null; // close the dropdown
    }
}

//=============================================================================
// Create modal, which reserves a virtual machine and deploys image and
// public-key on it
//=============================================================================
function openCreateModal() {
    showCreateModal.value = true;
}
function cancelCreateModal() {
    showCreateModal.value = false;
}
async function acceptCreateModal() {
    await fetchVirtualMachines();
    cancelCreateModal();
}

//=============================================================================
// Info modal
//=============================================================================
function openInfoModal(virtualMachine: VirtualMachineBasicResp) {
    virtualMachineToShow.value = virtualMachine;
    showInfoModal.value = true;
    openDropdown.value = null;
}
function cancelInfoModal() {
    showInfoModal.value = false;
    virtualMachineToShow.value = null;
    openDropdown.value = null;
}

//=============================================================================
// Delete modal
//=============================================================================
function openDeleteModal(virtualMachine: VirtualMachineBasicResp) {
    virtualMachineToDelete.value = virtualMachine;
    showDeleteModal.value = true;
    openDropdown.value = null;
}
function cancelDeleteModal() {
    showDeleteModal.value = false;
    virtualMachineToDelete.value = null;
    openDropdown.value = null; // close any open action dropdown
}
async function acceptDeleteModal() {
    await fetchVirtualMachines();
    cancelDeleteModal();
}

//=============================================================================
// Power-state, which is changed by a new task on the virtual machine
//=============================================================================
type PowerAction = "start" | "stop" | "reboot";
const powerFunctions: Record<
    PowerAction,
    (toriiPort: number, virtualMachineUuid: string) => Promise<unknown>
> = {
    start: sakura.startVirtualMachine,
    stop: sakura.stopVirtualMachine,
    reboot: sakura.rebootVirtualMachine,
};

/**
 * The sakura is only reachable through the torii, so the port of the
 * virtual-machine is resolved first. The task runs in the background and can
 * be followed in the task-view.
 */
async function changePowerState(
    virtualMachine: VirtualMachineBasicResp,
    action: PowerAction,
) {
    openDropdown.value = null;
    try {
        const resp = await hanami.getVirtualMachine(virtualMachine.uuid);
        await powerFunctions[action](resp.torii_port, virtualMachine.uuid);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            `Failed to ${action} virtual machine`,
        );
    }
}

//=============================================================================
// Snapshot modals, which both create a new task on the virtual machine
//=============================================================================
/**
 * The sakura is only reachable through the torii, so the port of the
 * virtual-machine has to be resolved, before a snapshot modal can be shown.
 */
async function prepareSnapshotModal(
    virtualMachine: VirtualMachineBasicResp,
): Promise<boolean> {
    openDropdown.value = null;
    try {
        const resp = await hanami.getVirtualMachine(virtualMachine.uuid);
        snapshotToriiPort.value = resp.torii_port;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual machine",
        );
        return false;
    }
    snapshotVirtualMachineUuid.value = virtualMachine.uuid;
    return true;
}
async function openSnapshotSaveModal(virtualMachine: VirtualMachineBasicResp) {
    if (await prepareSnapshotModal(virtualMachine)) {
        showSnapshotSaveModal.value = true;
    }
}
async function openSnapshotRestoreModal(
    virtualMachine: VirtualMachineBasicResp,
) {
    if (await prepareSnapshotModal(virtualMachine)) {
        showSnapshotRestoreModal.value = true;
    }
}
function closeSnapshotModals() {
    showSnapshotSaveModal.value = false;
    showSnapshotRestoreModal.value = false;
    snapshotVirtualMachineUuid.value = null;
    snapshotToriiPort.value = 0;
}

//=============================================================================
// Listener
//=============================================================================
onMounted(fetchVirtualMachines);

onMounted(() => {
    window.addEventListener("click", handleClickOutside);
    statePollTimer = setInterval(pollReservedVirtualMachines, 1000);
});

onBeforeUnmount(() => {
    window.removeEventListener("click", handleClickOutside);
    if (statePollTimer !== null) {
        clearInterval(statePollTimer);
        statePollTimer = null;
    }
});
</script>

<style scoped>
/* the uuid has always the same length, so it is not wrapped */
td:first-child {
    white-space: nowrap;
}

/* Columns 2 through n-2 share remaining space equally */
th:not(:first-child):not(.state-column):not(.resource-column):not(.address-column):not(:last-child),
td:not(:first-child):not(.state-column):not(.resource-column):not(.address-column):not(:last-child) {
    width: 30%;
}

/* wider than the default, so "Restore from snapshot" fits in one line */
.table-dropdown-menu {
    min-width: 13rem;
}

.address-column {
    width: 15%;
}

.resource-column {
    width: 10%;
    min-width: 7rem;
    white-space: nowrap;
    text-align: right;
}

/* only as wide as the header, with the light in the center */
.state-column {
    width: 1%;
    white-space: nowrap;
    text-align: center;
    /* more space between the disk and the state */
    padding-left: 2rem;
}

/* block instead of inline-block, so it isn't aligned to the text-baseline, but
centered by the cell */
.state-light {
    display: block;
    margin: 0 auto;
    width: 0.9rem;
    height: 0.9rem;
    border-radius: 50%;
    background-color: var(--color-text-light);
}

.state-created {
    background-color: #66bb6a;
    box-shadow: 0 0 0.4rem #66bb6a;
}

.state-reserved {
    background-color: #ffca28;
    box-shadow: 0 0 0.4rem #ffca28;
}

.state-error {
    background-color: #ff4d4d;
    box-shadow: 0 0 0.4rem #ff4d4d;
}
</style>
