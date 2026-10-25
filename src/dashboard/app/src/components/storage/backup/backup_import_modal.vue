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
        <div class="modal backup-import-modal">
            <div class="modal-topbar">
                <span>Import backup</span>
            </div>
            <div class="modal-content">
                <p>
                    Imports a backup, which was copied onto a backup-target from
                    outside, for example from another deployment for a
                    disaster-recovery. Afterwards it can be restored like every
                    other backup.
                </p>
                <div class="field-row">
                    <label for="backup-target">Backup-Target: </label>
                    <select
                        id="backup-target"
                        v-model="hostUuid"
                        class="select-dropdown"
                        @change="scan"
                    >
                        <option value="" disabled>Select a backup-target</option>
                        <option
                            v-for="host in backupTargets"
                            :key="host.uuid"
                            :value="host.uuid"
                        >
                            {{ host.name }} ({{ host.host_address }})
                        </option>
                    </select>
                </div>
                <p v-if="backupTargets.length === 0" class="hint-msg">
                    No onsen-host is marked as backup-target.
                </p>

                <div v-if="hostUuid" class="field-row">
                    <label for="backup">Backup: </label>
                    <select id="backup" v-model="backupUuid" class="select-dropdown">
                        <option value="" disabled>
                            {{
                                scanned.length > 0
                                    ? "Select a backup"
                                    : "No unknown backups found"
                            }}
                        </option>
                        <!-- a backup without manifest is incomplete and can't be imported -->
                        <option
                            v-for="entry in scanned"
                            :key="entry.uuid"
                            :value="entry.uuid"
                            :disabled="!entry.has_manifest"
                        >
                            {{ entry.uuid }}{{ entry.has_manifest ? "" : " (incomplete)" }}
                        </option>
                    </select>
                </div>

                <template v-if="backupUuid">
                    <div>
                        <input
                            v-model="passphrase"
                            type="password"
                            placeholder="Passphrase of the backup"
                            autocomplete="off"
                            :class="{ invalid_input: passphraseError }"
                        />
                        <p v-if="passphraseError" class="error-msg">
                            The passphrase is required
                        </p>
                    </div>
                    <div>
                        <input
                            v-model="name"
                            type="text"
                            placeholder="Name (default: imported-<uuid>)"
                            :class="{ invalid_input: nameError }"
                        />
                        <p v-if="nameError" class="error-msg">
                            Name must have between 4 and 127 characters
                        </p>
                    </div>
                </template>
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
import { ref, onMounted } from "vue";

import { isekai, ryokan } from "@/api";
import type { BackupScanEntry, OnsenHostBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const backupTargets = ref<OnsenHostBasicResp[]>([]);
const scanned = ref<BackupScanEntry[]>([]);
const hostUuid = ref<string>("");
const backupUuid = ref<string>("");
const passphrase = ref<string>("");
const name = ref<string>("");
const passphraseError = ref(false);
const nameError = ref(false);
const submitting = ref(false);
const errorPopupMsg = ref<string>("");

async function fetchBackupTargets() {
    try {
        const hosts = await ryokan.listOnsenHosts();
        backupTargets.value = hosts.filter((host) => host.is_backup_target);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load onsen-hosts");
    }
}

async function scan() {
    scanned.value = [];
    backupUuid.value = "";
    if (!hostUuid.value) return;
    try {
        scanned.value = await isekai.scanBackupTarget(hostUuid.value);
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to scan backup-target");
    }
}

async function handleAccept() {
    if (!hostUuid.value || !backupUuid.value) return;
    passphraseError.value = passphrase.value.length === 0;
    nameError.value =
        name.value.length > 0 && (name.value.length < 4 || name.value.length > 127);
    if (passphraseError.value || nameError.value) return;

    submitting.value = true;
    try {
        await isekai.importBackup({
            onsen_host_uuid: hostUuid.value,
            backup_uuid: backupUuid.value,
            passphrase: passphrase.value,
            name: name.value || undefined,
        });
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to import backup");
    } finally {
        submitting.value = false;
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchBackupTargets);
</script>

<style scoped>
.backup-import-modal {
    width: 38rem;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

.field-row {
    display: grid;
    grid-template-columns: 8rem 1fr;
    align-items: center;
}
</style>
