<!--
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
-->

<template>
    <div class="modal-overlay" @click.self="cancel">
        <div class="modal migrate-modal">
            <div class="modal-topbar">
                <span>Migrate virtual machine</span>
            </div>
            <div class="modal-content">
                <p>
                    Moves the virtual machine
                    <strong>{{ virtual_machine?.name }}</strong> to another
                    sakura-host. It is shut down, its disk is transferred to the
                    new host and it is booted there again, if it was running
                    before. It keeps its address, its proxy-port and its network
                    filters.
                </p>
                <br />
                <p>
                    The migration runs in the background, while the state of the
                    virtual machine is <code>MIGRATING</code>. If it fails, the
                    virtual machine is started on its current host again.
                </p>
                <div class="field-row">
                    <label for="target-host">Target host: </label>
                    <select
                        id="target-host"
                        v-model="targetHostUuid"
                        class="select-dropdown"
                        :class="{ invalid_input: targetError }"
                    >
                        <option value="" disabled>Select a host</option>
                        <option
                            v-for="host in hosts"
                            :key="host.uuid"
                            :value="host.uuid"
                            :disabled="!hasRoom(host)"
                        >
                            {{ hostLabel(host) }}
                        </option>
                    </select>
                </div>
                <p v-if="targetError" class="error-msg">
                    Select the host, which runs the virtual machine afterwards
                </p>
                <p class="hint-msg">
                    Only hosts with enough free cores, memory and disk for the
                    virtual machine can be selected. The new host needs the same
                    kind of isolation as the current one, which is rejected as
                    target.
                </p>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        :disabled="submitting"
                        @click="handleAccept"
                    >
                        <img :src="icons.acceptIcon" alt="Accept" />
                    </button>
                    <button class="icon-button" @click="cancel">
                        <img :src="icons.cancelIcon" alt="Cancel" />
                    </button>
                </div>
            </div>
        </div>
    </div>
    <div v-if="errorPopupMsg" class="error-popup">
        <button class="error-close-btn" @click="errorPopupMsg = ''">✕</button>
        {{ errorPopupMsg }}
    </div>
</template>

<script lang="ts" setup>
import { ref, onMounted } from "vue";

import { hanami } from "@/api";
import type { SakuraHostBasicResp, VirtualMachineBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine: VirtualMachineBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept", virtualMachineUuid: string): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const hosts = ref<SakuraHostBasicResp[]>([]);
const targetHostUuid = ref<string>("");
const targetError = ref(false);
// prevents a second migration by a double-click, while the first is requested
const submitting = ref(false);

async function fetchHosts() {
    try {
        const list = await hanami.listSakuraHosts();
        hosts.value = list.sort((a, b) => a.name.localeCompare(b.name));
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load hosts");
    }
}

/** Free resources of a host. The memory is given in MiB, the disk in GiB. */
function freeResources(host: SakuraHostBasicResp) {
    return {
        cores: host.number_of_cores - host.used_number_of_cores,
        memory: host.memory_size - host.amount_of_used_memory,
        disk: host.disk_space - host.amount_of_used_disk_space,
    };
}

/** Checks, if the host has enough free resources for the virtual machine. */
function hasRoom(host: SakuraHostBasicResp): boolean {
    const vm = props.virtual_machine;
    if (!vm) return false;
    const free = freeResources(host);
    return (
        free.cores >= vm.number_of_cores &&
        free.memory >= vm.memory_size &&
        free.disk >= vm.disk_size
    );
}

function hostLabel(host: SakuraHostBasicResp): string {
    const free = freeResources(host);
    const memoryGiB = Math.round((free.memory / 1024) * 10) / 10;
    const isolated = host.is_host_isolated ? ", isolated" : "";
    return (
        `${host.name} (free: ${free.cores} cores, ${memoryGiB} GiB memory, ` +
        `${free.disk} GiB disk${isolated})`
    );
}

async function handleAccept() {
    targetError.value = targetHostUuid.value === "";
    if (targetError.value || !props.virtual_machine || submitting.value) {
        return;
    }

    submitting.value = true;
    try {
        await hanami.migrateVirtualMachine(props.virtual_machine.uuid, {
            target_host_uuid: targetHostUuid.value,
        });
        emit("accept", props.virtual_machine.uuid);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to migrate virtual machine",
        );
    } finally {
        submitting.value = false;
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchHosts);
</script>

<style scoped>
.migrate-modal {
    width: 36rem;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

.field-row {
    display: grid;
    grid-template-columns: 7rem 1fr;
    align-items: center;
    margin-top: 1rem;
}
</style>
