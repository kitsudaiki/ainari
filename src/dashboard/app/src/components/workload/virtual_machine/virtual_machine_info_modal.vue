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
        <div class="modal virtual-machine-info-modal">
            <div class="modal-topbar">
                <span>Info</span>
            </div>
            <div class="modal-content">
                <table v-if="info">
                    <tbody>
                        <tr>
                            <td>UUID</td>
                            <td>{{ info.uuid }}</td>
                        </tr>
                        <tr>
                            <td>Name</td>
                            <td>{{ info.name }}</td>
                        </tr>
                        <tr>
                            <td>Is Created</td>
                            <td>
                                <div class="bool-icon">
                                    <img
                                        v-if="info.is_created"
                                        :src="icons.acceptIcon"
                                        alt="True"
                                    />
                                    <img
                                        v-else
                                        :src="icons.cancelIcon"
                                        alt="False"
                                    />
                                </div>
                            </td>
                        </tr>
                        <tr>
                            <td>Number of Cores</td>
                            <td>{{ info.number_of_cores }}</td>
                        </tr>
                        <tr>
                            <td>Memory Size</td>
                            <td>{{ memorySize }}</td>
                        </tr>
                        <tr>
                            <td>Image-UUID</td>
                            <td>{{ info.image_uuid }}</td>
                        </tr>
                        <tr>
                            <td>Network-UUID</td>
                            <td>{{ info.network_uuid }}</td>
                        </tr>
                        <tr>
                            <td>Internal IP</td>
                            <td>{{ info.internal_ip }}</td>
                        </tr>
                        <tr>
                            <td>Torii-Port</td>
                            <td>{{ info.torii_port }}</td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ info.created_at }}</td>
                        </tr>
                        <tr>
                            <td>Created By</td>
                            <td>{{ info.created_by }}</td>
                        </tr>
                        <tr>
                            <td>Updated At</td>
                            <td>{{ info.updated_at }}</td>
                        </tr>
                        <tr>
                            <td>Updated By</td>
                            <td>{{ info.updated_by }}</td>
                        </tr>
                    </tbody>
                </table>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
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
import { ref, computed, onMounted } from "vue";

import { hanami } from "@/api";
import type { VirtualMachineBasicResp, VirtualMachineResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine: VirtualMachineBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();

const info = ref<VirtualMachineResp | null>(null);
const errorPopupMsg = ref<string>("");

// the api provides the memory-size in bytes, which is hard to read for bigger machines
const memorySize = computed(() => {
    if (!info.value) return "";
    const mib = info.value.memory_size / (1024 * 1024);
    return `${mib} MiB`;
});

async function fetchInfo(uuid: string) {
    try {
        const data = await hanami.getVirtualMachine(uuid);
        data.created_at = common.formatDateTime(data.created_at);
        data.updated_at = common.formatDateTime(data.updated_at);
        info.value = data;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load virtual-machine-info",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.virtual_machine) {
        fetchInfo(props.virtual_machine.uuid);
    }
});
</script>

<style scoped>
.virtual-machine-info-modal {
    height: 34rem;
    width: 40rem;
}
</style>
