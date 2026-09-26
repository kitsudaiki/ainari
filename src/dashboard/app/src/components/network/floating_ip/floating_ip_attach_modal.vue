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
        <div class="modal floating-ip-attach-modal">
            <div class="modal-topbar">
                <span>Attach floating IP</span>
            </div>
            <div class="modal-content">
                <strong>Floating IP: {{ floating_ip?.floating_ip }}</strong>
                <br />
                <br />
                <div class="field-row">
                    <label for="virtual_machine">Virtual machine: </label>
                    <select
                        id="virtual_machine"
                        v-model="virtualMachineUuid"
                        class="select-dropdown"
                        :class="{ invalid_input: virtualMachineError }"
                    >
                        <option value="" disabled>Select a virtual machine</option>
                        <option
                            v-for="virtualMachine in virtualMachines"
                            :key="virtualMachine.uuid"
                            :value="virtualMachine.uuid"
                        >
                            {{ virtualMachine.name }}
                        </option>
                    </select>
                </div>
                <p v-if="virtualMachineError" class="error-msg">
                    A virtual machine must be selected
                </p>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        @click="handleAccept(floating_ip?.uuid)"
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
import type { FloatingIpBasicResp, VirtualMachineBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    floating_ip: FloatingIpBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();
const errorPopupMsg = ref<string>("");
const virtualMachineError = ref(false);
const virtualMachineUuid = ref<string>("");
const virtualMachines = ref<VirtualMachineBasicResp[]>([]);

async function fetchVirtualMachines() {
    try {
        virtualMachines.value = await hanami.listVirtualMachines();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual machines",
        );
    }
}

async function handleAccept(floating_ip_uuid: string | undefined) {
    if (!floating_ip_uuid) return;
    virtualMachineError.value = virtualMachineUuid.value === "";
    if (virtualMachineError.value) return;

    try {
        await hanami.attachFloatingIp(floating_ip_uuid, {
            virtual_machine_uuid: virtualMachineUuid.value,
        });
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to attach floating IP",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchVirtualMachines);
</script>

<style scoped>
.floating-ip-attach-modal {
    width: 32rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}

.field-row {
    display: grid;
    grid-template-columns: 10rem 18rem; /* fixed input width */
    align-items: center;
}
</style>
