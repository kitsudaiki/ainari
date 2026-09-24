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
        <div class="modal snapshot-restore-modal">
            <div class="modal-topbar">
                <span>Restore snapshot</span>
            </div>
            <div class="modal-content">
                <p>
                    Creates a task, which restores the virtual machine from an
                    existing snapshot.
                </p>
                <br />
                <div>
                    <input
                        v-model="name"
                        type="text"
                        placeholder="Task-Name"
                        :class="{ invalid_input: nameError }"
                    />
                    <p v-if="nameError" class="error-msg">
                        Task-Name must be at least 4 characters
                    </p>
                </div>
                <br />
                <div class="field-row">
                    <label for="snapshot">Snapshot: </label>
                    <select
                        id="snapshot"
                        v-model="selectedSnapshotUuid"
                        class="select-dropdown"
                        :class="{ invalid_input: snapshotError }"
                    >
                        <option value="" disabled>Select a snapshot</option>
                        <option
                            v-for="snapshot in snapshots"
                            :key="snapshot.uuid"
                            :value="snapshot.uuid"
                        >
                            {{ snapshot.name }}
                        </option>
                    </select>
                </div>
                <p v-if="snapshotError" class="error-msg">
                    <template v-if="snapshots.length === 0">
                        No snapshots available.
                    </template>
                    <template v-else>A snapshot must be selected</template>
                </p>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button class="icon-button" @click="handleAccept">
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

import { ryokan, sakura } from "@/api";
import type { SnapshotBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    virtual_machine_uuid: string | null;
    torii_port: number;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const nameError = ref(false);
const snapshotError = ref(false);
const name = ref<string>("");
const snapshots = ref<SnapshotBasicResp[]>([]);
const selectedSnapshotUuid = ref<string>("");

async function fetchSnapshots() {
    try {
        snapshots.value = await ryokan.listSnapshots();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load snapshots",
        );
    }
}

async function handleAccept() {
    nameError.value = name.value.length < 4;
    snapshotError.value = selectedSnapshotUuid.value === "";

    if (nameError.value || snapshotError.value || !props.virtual_machine_uuid) {
        return;
    }

    try {
        await sakura.createSnapshotRestoreTask(
            props.torii_port,
            props.virtual_machine_uuid,
            {
                name: name.value,
                snapshot_uuid: selectedSnapshotUuid.value,
            },
        );

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to create snapshot-restore-task",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchSnapshots);
</script>

<style scoped>
.snapshot-restore-modal {
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
