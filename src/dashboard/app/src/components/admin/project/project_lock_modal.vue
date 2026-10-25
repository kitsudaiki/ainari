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
        <div class="modal project-lock-modal">
            <div class="modal-topbar">
                <span>Lock project: {{ project?.id }}</span>
            </div>
            <div class="modal-content">
                <p>
                    The resources of a locked project can only be read. A
                    project is locked by its backups and restores, and stays
                    locked after a failed restore, until the restore is repeated
                    with force or it is unlocked here.
                </p>
                <template v-if="loaded">
                    <table v-if="lock">
                        <tbody>
                            <tr>
                                <td>Reason</td>
                                <td>{{ lock.lock_reason }}</td>
                            </tr>
                            <tr>
                                <td>Locked by</td>
                                <td>
                                    {{
                                        lock.locked_by_job === NIL_UUID
                                            ? "an admin"
                                            : `job ${lock.locked_by_job}`
                                    }}
                                </td>
                            </tr>
                            <tr>
                                <td>Locked at</td>
                                <td>{{ common.formatDateTime(lock.locked_at) }}</td>
                            </tr>
                        </tbody>
                    </table>
                    <p v-if="lock">
                        <strong>
                            Accept removes the lock
                            {{
                                lock.locked_by_job === NIL_UUID
                                    ? ""
                                    : "of the running or failed job"
                            }}.
                        </strong>
                    </p>
                    <div v-else>
                        <input
                            v-model="reason"
                            type="text"
                            placeholder="Reason, like a maintenance"
                            :class="{ invalid_input: reasonError }"
                        />
                        <p v-if="reasonError" class="error-msg">
                            The reason is required
                        </p>
                    </div>
                </template>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        :disabled="!loaded"
                        :title="lock ? 'Unlock the project' : 'Lock the project'"
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

import { miko } from "@/api";
import type { ProjectBasicResp, ProjectLock } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

/** job of the lock of an admin, which no backup or restore can take over */
const NIL_UUID = "00000000-0000-0000-0000-000000000000";

interface Props {
    project: ProjectBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const lock = ref<ProjectLock | null>(null);
const loaded = ref(false);
const reason = ref<string>("");
const reasonError = ref(false);
const errorPopupMsg = ref<string>("");

async function fetchLock(projectId: string) {
    try {
        lock.value = (await miko.getProject(projectId)).lock ?? null;
        loaded.value = true;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load project");
    }
}

async function handleAccept() {
    if (!props.project || !loaded.value) return;

    try {
        if (lock.value) {
            await miko.unlockProject(props.project.id);
        } else {
            reasonError.value = reason.value.trim().length === 0;
            if (reasonError.value) return;
            await miko.lockProject(props.project.id, { reason: reason.value.trim() });
        }
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            lock.value ? "Failed to unlock project" : "Failed to lock project",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.project) {
        fetchLock(props.project.id);
    }
});
</script>

<style scoped>
.project-lock-modal {
    width: 34rem;
}
</style>
