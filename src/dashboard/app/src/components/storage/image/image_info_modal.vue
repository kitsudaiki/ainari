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
        <div class="modal image-info-modal">
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
                            <td>Snapshot</td>
                            <td>{{ info.is_snapshot ? "Yes" : "No" }}</td>
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
import { ref, onMounted } from "vue";

import { ryokan } from "@/api";
import type { ImageBasicResp, ImageResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    image: ImageBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();

const info = ref<ImageResp | null>(null);
const errorPopupMsg = ref<string>("");

async function fetchInfo(uuid: string) {
    try {
        const data = await ryokan.getImage(uuid);
        data.created_at = common.formatDateTime(data.created_at);
        data.updated_at = common.formatDateTime(data.updated_at);
        info.value = data;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load image-info");
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.image) {
        fetchInfo(props.image.uuid);
    }
});
</script>

<style scoped>
.image-info-modal {
    width: 40rem;
}
</style>
