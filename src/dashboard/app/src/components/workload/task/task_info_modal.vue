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
        <div class="modal task-info-modal">
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
                            <td>Type</td>
                            <td>{{ info.task_type }}</td>
                        </tr>
                        <tr>
                            <td>State</td>
                            <td>{{ info.state }}</td>
                        </tr>
                        <tr>
                            <td>Queued At</td>
                            <td>{{ formatted(info.queued_at) }}</td>
                        </tr>
                        <tr>
                            <td>Started At</td>
                            <td>{{ formatted(info.started_at) }}</td>
                        </tr>
                        <tr>
                            <td>Finished At</td>
                            <td>{{ formatted(info.finished_at) }}</td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ formatted(info.created_at) }}</td>
                        </tr>
                        <tr>
                            <td>Created By</td>
                            <td>{{ info.created_by }}</td>
                        </tr>
                    </tbody>
                </table>

                <div v-if="info && info.messages.length > 0">
                    <br />
                    <strong>Messages</strong>
                    <ul class="message-list">
                        <li
                            v-for="(message, index) in info.messages"
                            :key="index"
                        >
                            {{ message }}
                        </li>
                    </ul>
                </div>
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
import { ref, onMounted, onBeforeUnmount } from "vue";

import { sakura } from "@/api";
import type { TaskBasicResp, TaskResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine_uuid: string | null;
    torii_port: number;
    task: TaskBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();

const info = ref<TaskResp | null>(null);
const errorPopupMsg = ref<string>("");

// a running task keeps adding messages and timestamps, so the info is polled
let refreshInterval: number | undefined;

/** The timestamps of a task stay empty until it reaches the matching state. */
function formatted(timestamp: string | null): string {
    if (!timestamp) return "-";
    return common.formatDateTime(timestamp);
}

async function fetchInfo() {
    if (!props.task || !props.virtual_machine_uuid) return;

    try {
        info.value = await sakura.getTask(
            props.torii_port,
            props.virtual_machine_uuid,
            props.task.uuid,
        );
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load task-info");
    }
}

function cancel() {
    emit("cancel");
}

onMounted(async () => {
    await fetchInfo();
    refreshInterval = window.setInterval(fetchInfo, 2000);
});

onBeforeUnmount(() => {
    if (refreshInterval) clearInterval(refreshInterval);
});
</script>

<style scoped>
.task-info-modal {
    width: 40rem;
}

.message-list {
    margin: 0.5rem 0 0 1rem;
    font-family: monospace;
    font-size: 0.85rem;
}
</style>
