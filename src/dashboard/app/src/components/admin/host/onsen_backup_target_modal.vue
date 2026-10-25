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
        <div class="modal onsen-backup-target-modal">
            <div class="modal-topbar">
                <span>Set backup target: {{ host?.name }}</span>
            </div>
            <div class="modal-content">
                <p>
                    A backup-target only stores the backups of isekai and never
                    normal images or snapshots. A host can only be marked, while
                    no images or snapshots are stored on it, and only be
                    unmarked, while no backups are stored on it.
                </p>
                <br />
                <div>
                    <label class="checkbox-label">
                        <input type="checkbox" v-model="isBackupTarget" />
                        Backup target
                    </label>
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

import { ryokan } from "@/api";
import type { OnsenHostBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    host: OnsenHostBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

// the flag is edited on a copy, so that a cancel leaves the table untouched
const isBackupTarget = ref(props.host?.is_backup_target ?? false);
const errorPopupMsg = ref<string>("");

async function handleAccept() {
    if (!props.host) return;

    // nothing to change
    if (isBackupTarget.value === props.host.is_backup_target) {
        emit("cancel");
        return;
    }

    try {
        await ryokan.setOnsenHostBackupTarget(props.host.uuid, {
            is_backup_target: isBackupTarget.value,
        });
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to set backup target",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.onsen-backup-target-modal {
    width: 30rem;
}
</style>
