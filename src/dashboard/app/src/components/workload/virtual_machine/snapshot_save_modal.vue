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
                    root-disk of the virtual machine as a new image, which is
                    marked as snapshot. The virtual machine is paused, while its
                    root-disk is copied.
                </p>
                <br />
                <p>
                    Data, which was written shortly before, can still be in the
                    memory of the virtual machine and is then missing in the
                    snapshot. Run <code>sync</code> inside the virtual machine
                    right before saving the snapshot.
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
                <div class="field-row">
                    <label for="secret">Secret: </label>
                    <select id="secret" v-model="secretUuid" class="select-dropdown">
                        <option value="">None (generate new secret)</option>
                        <option
                            v-for="secret in secrets"
                            :key="secret.uuid"
                            :value="secret.uuid"
                        >
                            {{ secret.name }}
                        </option>
                    </select>
                </div>
                <p class="hint-msg">
                    The snapshot is encrypted with the selected secret. Without a
                    selection, a new secret is generated for the snapshot.
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

import { omamori, sakura } from "@/api";
import type { SecretBasicResp } from "@/api";
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
const secretUuid = ref<string>("");
const secrets = ref<SecretBasicResp[]>([]);

async function fetchSecrets() {
    try {
        secrets.value = await omamori.listSecrets();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load secrets");
    }
}

async function handleAccept() {
    nameError.value = name.value.length < 4;
    if (nameError.value || !props.virtual_machine_uuid) {
        return;
    }

    try {
        await sakura.createSnapshotSaveTask(
            props.torii_port,
            props.virtual_machine_uuid,
            {
                name: name.value,
                secret_uuid: secretUuid.value || undefined,
            },
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

onMounted(fetchSecrets);
</script>

<style scoped>
.snapshot-save-modal {
    width: 30rem;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

.field-row {
    display: grid;
    grid-template-columns: 6rem 22rem;
    align-items: center;
    margin-top: 1rem;
}
</style>
