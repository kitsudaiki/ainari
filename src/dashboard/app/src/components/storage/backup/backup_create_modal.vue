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
        <div class="modal backup-create-modal">
            <div class="modal-topbar">
                <span v-if="virtualMachine">
                    Create backup: {{ virtualMachine.name }}
                </span>
                <span v-else-if="projectId">Create backup: {{ projectId }}</span>
                <span v-else>Create project backup</span>
            </div>
            <div class="modal-content">
                <p v-if="virtualMachine">
                    Saves the virtual machine with a copy of its root-disk, its
                    public-key, its network and its network-filters.
                </p>
                <p v-else>
                    Saves the whole project: its quota, secrets, public-keys,
                    images, networks, virtual machines, network-filters and
                    floating IPs, together with a copy of the root-disk of every
                    virtual machine. Users and their assignments are not part of
                    the backup.
                </p>
                <p class="hint-msg">
                    The project is locked, until the backup is finished, so its
                    resources can only be read meanwhile. The virtual machines
                    are only paused shortly, unless they are shut down for the
                    backup.
                </p>
                <div>
                    <input
                        v-model="form.name"
                        type="text"
                        placeholder="Backup-Name"
                        :class="{ invalid_input: nameError }"
                    />
                    <p v-if="nameError" class="error-msg">
                        Backup-Name must have between 4 and 127 characters
                    </p>
                </div>
                <div>
                    <input
                        v-model="form.passphrase"
                        type="password"
                        placeholder="Passphrase of the backup"
                        autocomplete="new-password"
                        :class="{ invalid_input: passphraseError }"
                    />
                    <p v-if="passphraseError" class="error-msg">
                        Passphrase must be at least
                        {{ MIN_BACKUP_PASSPHRASE_LENGTH }} characters
                    </p>
                </div>
                <div>
                    <input
                        v-model="form.confirmPassphrase"
                        type="password"
                        placeholder="Confirm passphrase"
                        autocomplete="new-password"
                        :class="{ invalid_input: confirmError }"
                    />
                    <p v-if="confirmError" class="error-msg">
                        Passphrase did not match
                    </p>
                </div>
                <p class="hint-msg">
                    The passphrase encrypts the backup and is never stored. A
                    backup can't be restored without it.
                </p>
                <label class="checkbox-label">
                    <input type="checkbox" v-model="form.stopVms" />
                    {{
                        virtualMachine
                            ? "Shut the virtual machine down for the backup"
                            : "Shut the running virtual machines down for the backup"
                    }}
                </label>
                <label v-if="!virtualMachine" class="checkbox-label">
                    <input type="checkbox" v-model="form.includeImages" />
                    Include the images and snapshots of the project
                </label>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        :disabled="submitting"
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
import { ref, reactive } from "vue";

import { isekai, miko, MIN_BACKUP_PASSPHRASE_LENGTH } from "@/api";
import type { BackupJobResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    /** virtual machine, which is saved, or null for a backup of the whole project */
    virtualMachine?: { uuid: string; name: string } | null;
    /**
     * project, which is saved, if it is not the project of the login. Isekai saves the project of
     * the token, so an admin gets a token for this project first.
     */
    projectId?: string | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = withDefaults(defineProps<Props>(), { virtualMachine: null, projectId: null });
const emit = defineEmits<{
    (e: "accept", result: BackupJobResp): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    name: "",
    passphrase: "",
    confirmPassphrase: "",
    stopVms: false,
    includeImages: true,
});
const nameError = ref(false);
const passphraseError = ref(false);
const confirmError = ref(false);
const submitting = ref(false);
const errorPopupMsg = ref<string>("");

async function handleAccept() {
    nameError.value = form.name.length < 4 || form.name.length > 127;
    passphraseError.value = form.passphrase.length < MIN_BACKUP_PASSPHRASE_LENGTH;
    confirmError.value = form.passphrase !== form.confirmPassphrase;
    if (nameError.value || passphraseError.value || confirmError.value) {
        return;
    }

    submitting.value = true;
    try {
        const result = props.virtualMachine
            ? await isekai.createVirtualMachineBackup({
                  virtual_machine_uuid: props.virtualMachine.uuid,
                  name: form.name,
                  passphrase: form.passphrase,
                  stop_vm: form.stopVms,
              })
            : await isekai.createProjectBackup(
                  {
                      name: form.name,
                      passphrase: form.passphrase,
                      stop_vms: form.stopVms,
                      include_images: form.includeImages,
                  },
                  props.projectId
                      ? (await miko.renewTokenForProjectAdmin(props.projectId)).access_token
                      : undefined,
              );
        emit("accept", result);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to create backup");
    } finally {
        submitting.value = false;
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.backup-create-modal {
    width: 34rem;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}
</style>
