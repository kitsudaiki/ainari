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
        <div class="modal backup-job-info-modal">
            <div class="modal-topbar">
                <span>Job</span>
            </div>
            <div class="modal-content">
                <table v-if="info">
                    <tbody>
                        <tr>
                            <td>UUID</td>
                            <td>{{ info.uuid }}</td>
                        </tr>
                        <tr>
                            <td>Type</td>
                            <td>{{ info.job_type }}</td>
                        </tr>
                        <tr>
                            <td>Backup</td>
                            <td>{{ info.backup_uuid }}</td>
                        </tr>
                        <tr>
                            <td>Project</td>
                            <td>{{ info.project_id }}</td>
                        </tr>
                        <tr>
                            <td>State</td>
                            <td>{{ info.state }}</td>
                        </tr>
                        <tr>
                            <td>Step</td>
                            <td>{{ info.step }}</td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ common.formatDateTime(info.created_at) }}</td>
                        </tr>
                        <tr>
                            <td>Created By</td>
                            <td>{{ info.created_by }}</td>
                        </tr>
                        <tr>
                            <td>Started At</td>
                            <td>{{ formatOptional(info.started_at) }}</td>
                        </tr>
                        <tr>
                            <td>Finished At</td>
                            <td>{{ formatOptional(info.finished_at) }}</td>
                        </tr>
                    </tbody>
                </table>

                <div v-if="info && info.messages.length > 0">
                    <p class="job-title">Messages</p>
                    <pre class="job-messages">{{ info.messages.join("\n") }}</pre>
                </div>

                <div v-if="info && info.id_map.length > 0">
                    <p class="job-title">Resources with a new UUID</p>
                    <table class="job-table">
                        <tbody>
                            <tr v-for="entry in info.id_map" :key="entry.old_uuid">
                                <td>{{ entry.resource_type }}</td>
                                <td>{{ entry.old_uuid }}</td>
                                <td>→ {{ entry.new_uuid }}</td>
                            </tr>
                        </tbody>
                    </table>
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
import { ref, onMounted } from "vue";

import { isekai } from "@/api";
import type { JobResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    job: JobResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();
const info = ref<JobResp | null>(null);
const errorPopupMsg = ref<string>("");

function formatOptional(value: string | null): string {
    return value ? common.formatDateTime(value) : "-";
}

async function fetchInfo(uuid: string) {
    try {
        info.value = await isekai.getJob(uuid);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load job");
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.job) {
        fetchInfo(props.job.uuid);
    }
});
</script>

<style scoped>
.backup-job-info-modal {
    width: 46rem;
    max-height: 90vh;
}

.job-title {
    font-weight: 600;
    margin-bottom: 0.3rem;
}

.job-messages {
    white-space: pre-wrap;
    font-size: 0.85rem;
}

.job-table {
    width: 100%;
    font-size: 0.85rem;
}
</style>
