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
        <div class="modal vm-type-info-modal">
            <div class="modal-topbar">
                <span>Info</span>
            </div>
            <div class="modal-content">
                <table v-if="vm_type_info">
                    <tbody>
                        <tr>
                            <td>UUID</td>
                            <td>{{ vm_type_info.uuid }}</td>
                        </tr>
                        <tr>
                            <td>Name</td>
                            <td>{{ vm_type_info.name }}</td>
                        </tr>
                        <tr>
                            <td>Number of Cores</td>
                            <td>{{ vm_type_info.number_of_cores }}</td>
                        </tr>
                        <tr>
                            <td>Amount of Memory</td>
                            <td>{{ vm_type_info.amount_of_memory }} MiB</td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ vm_type_info.created_at }}</td>
                        </tr>
                        <tr>
                            <td>Created By</td>
                            <td>{{ vm_type_info.created_by }}</td>
                        </tr>
                        <tr>
                            <td>Updated At</td>
                            <td>{{ vm_type_info.updated_at }}</td>
                        </tr>
                        <tr>
                            <td>Updated By</td>
                            <td>{{ vm_type_info.updated_by }}</td>
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
import { ref, onMounted } from "vue";

import { hanami } from "@/api";
import type { VmTypeBasicResp, VmTypeResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

const vm_type_info = ref<VmTypeResp | null>(null);
const errorPopupMsg = ref<string>("");

interface Props {
    // only the uuid is needed, because all other infos are loaded by the modal itself
    vmType: Pick<VmTypeBasicResp, "uuid"> | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

async function fetchVmTypeInfo(uuid: string) {
    try {
        const data = await hanami.getVmType(uuid);
        data.created_at = common.formatDateTime(data.created_at);
        data.updated_at = common.formatDateTime(data.updated_at);
        vm_type_info.value = data;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load vm-type-info",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.vmType) {
        fetchVmTypeInfo(props.vmType.uuid);
    }
});
</script>

<style scoped>
.vm-type-info-modal {
    width: 40rem;
}
</style>
