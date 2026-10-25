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
        <div class="modal backup-info-modal">
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
                            <td>Kind</td>
                            <td>{{ backupKindLabel(info.kind) }}</td>
                        </tr>
                        <tr>
                            <td>Project</td>
                            <td>{{ info.project_id }}</td>
                        </tr>
                        <tr v-if="info.virtual_machine_uuid">
                            <td>Virtual Machine</td>
                            <td>{{ info.virtual_machine_uuid }}</td>
                        </tr>
                        <tr>
                            <td>State</td>
                            <td>{{ info.state }}</td>
                        </tr>
                        <tr>
                            <td>Files</td>
                            <td>{{ info.file_count ?? "-" }}</td>
                        </tr>
                        <tr>
                            <td>Size</td>
                            <td>
                                {{
                                    info.total_size !== null
                                        ? common.formatFileSize(info.total_size)
                                        : "-"
                                }}
                            </td>
                        </tr>
                        <tr>
                            <td>Backup-Target</td>
                            <td>{{ info.host_uuid ?? "-" }}</td>
                        </tr>
                        <tr>
                            <td>Ainari-Version</td>
                            <td>{{ info.ainari_version }}</td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ common.formatDateTime(info.created_at) }}</td>
                        </tr>
                        <tr>
                            <td>Created By</td>
                            <td>{{ info.created_by }}</td>
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

import { isekai } from "@/api";
import type { BackupKind, BackupResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    backup: BackupResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();
const info = ref<BackupResp | null>(null);
const errorPopupMsg = ref<string>("");

function backupKindLabel(kind: BackupKind): string {
    return kind === "PROJECT" ? "Project" : "Virtual Machine";
}

async function fetchInfo(uuid: string) {
    try {
        info.value = await isekai.getBackup(uuid);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load backup-info");
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.backup) {
        fetchInfo(props.backup.uuid);
    }
});
</script>

<style scoped>
.backup-info-modal {
    width: 40rem;
}
</style>
