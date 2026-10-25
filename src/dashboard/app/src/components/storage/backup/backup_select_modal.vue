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
        <div class="modal backup-select-modal">
            <div class="modal-topbar">
                <span>Restore backup: {{ projectId }}</span>
            </div>
            <div class="modal-content">
                <p>
                    Select the backup, which is restored into the project. The
                    plan of the restore is shown, before anything is changed.
                </p>
                <table v-if="backups.length > 0" class="select-table">
                    <thead>
                        <tr>
                            <th></th>
                            <th>Name</th>
                            <th>Kind</th>
                            <th>Created At</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr
                            v-for="backup in backups"
                            :key="backup.uuid"
                            @click="selectedUuid = backup.uuid"
                        >
                            <td>
                                <input
                                    type="radio"
                                    :value="backup.uuid"
                                    v-model="selectedUuid"
                                />
                            </td>
                            <td>{{ backup.name }}</td>
                            <td>
                                {{ backup.kind === "PROJECT" ? "Project" : "Virtual Machine" }}
                            </td>
                            <td>{{ common.formatDateTime(backup.created_at) }}</td>
                        </tr>
                    </tbody>
                </table>
                <p v-else-if="loaded">The project has no backups, which can be restored.</p>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        :disabled="!selectedUuid"
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

import { isekai } from "@/api";
import type { BackupResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    /** project, whose backups are listed */
    projectId: string;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "select", backup: BackupResp): void;
    (e: "cancel"): void;
}>();

const backups = ref<BackupResp[]>([]);
const selectedUuid = ref<string | null>(null);
const loaded = ref(false);
const errorPopupMsg = ref<string>("");

async function fetchBackups() {
    try {
        // only a sealed backup can be restored
        backups.value = (await isekai.listBackups(props.projectId)).filter(
            (backup) => backup.state === "READY",
        );
        loaded.value = true;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load backups");
    }
}

function handleAccept() {
    const backup = backups.value.find((entry) => entry.uuid === selectedUuid.value);
    if (backup) {
        emit("select", backup);
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchBackups);
</script>

<style scoped>
.backup-select-modal {
    width: 40rem;
    max-height: 90vh;
}

.select-table {
    width: 100%;
}

.select-table tbody tr {
    cursor: pointer;
}
</style>
