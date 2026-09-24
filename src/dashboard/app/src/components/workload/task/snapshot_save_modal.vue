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
        <div class="modal snapshot-save-modal">
            <div class="modal-topbar">
                <span>Save snapshot</span>
            </div>
            <div class="modal-content">
                <p>
                    Creates a task, which stores the current state of the
                    root-disk of the virtual machine as a new snapshot. The
                    virtual machine is paused, while its root-disk is copied.
                </p>
                <br />
                <div>
                    <input
                        v-model="name"
                        type="text"
                        placeholder="Snapshot-Name"
                        :class="{ invalid_input: nameError }"
                    />
                    <p v-if="nameError" class="error-msg">
                        Snapshot-Name must be at least 4 characters
                    </p>
                </div>
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
import { ref } from "vue";

import { sakura } from "@/api";
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
const name = ref<string>("");

async function handleAccept() {
    nameError.value = name.value.length < 4;
    if (nameError.value || !props.virtual_machine_uuid) {
        return;
    }

    try {
        await sakura.createSnapshotSaveTask(
            props.torii_port,
            props.virtual_machine_uuid,
            { name: name.value },
        );

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to create snapshot-save-task",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.snapshot-save-modal {
    width: 30rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
