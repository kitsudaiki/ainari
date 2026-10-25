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
        <div class="modal backup-restore-modal">
            <div class="modal-topbar">
                <span>Restore backup: {{ backup?.name }}</span>
            </div>

            <!-- first step: the options of the restore -->
            <div v-if="!plan" class="modal-content">
                <p>
                    The restore is calculated as dry-run first, so its plan can
                    be checked, before anything is changed.
                </p>
                <div>
                    <input
                        v-model="form.passphrase"
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
                        v-model="form.targetProjectId"
                        type="text"
                        :placeholder="`Target project (default: ${backup?.project_id})`"
                    />
                    <p class="hint-msg">
                        Restoring into another project gives all resources new
                        UUIDs. A project, which doesn't exist, is created, which
                        only admins can do.
                    </p>
                </div>
                <label class="checkbox-label">
                    <input type="checkbox" v-model="form.force" />
                    Overwrite the existing resources with the same UUID (force)
                </label>
                <label v-if="isProjectBackup" class="checkbox-label">
                    <input
                        type="checkbox"
                        v-model="form.prune"
                        :disabled="!form.force"
                    />
                    Delete the resources, which are not part of the backup
                    (prune, requires force)
                </label>
                <label class="checkbox-label">
                    <input type="checkbox" v-model="form.startVms" />
                    Start all restored virtual machines
                </label>
                <label v-if="!isProjectBackup" class="checkbox-label">
                    <input
                        type="checkbox"
                        v-model="form.includeDependencies"
                    />
                    Create the network of the virtual machine from the
                    backup, if it is missing
                </label>
            </div>

            <!-- second step: the plan of the dry-run, which is confirmed -->
            <div v-else class="modal-content">
                <div>
                    <button
                        class="back-button plan-back-button"
                        title="Change the options"
                        @click="plan = null"
                    >
                        ←
                    </button>
                </div>
                <p>
                    Target project: <strong>{{ plan.target_project_id }}</strong>
                    <span v-if="plan.create_project">
                        (is created by the restore)</span
                    >
                </p>

                <div v-if="plan.warnings.length > 0" class="plan-warnings">
                    <p v-for="warning in plan.warnings" :key="warning">
                        ⚠ {{ warning }}
                    </p>
                </div>

                <div v-if="plan.deleted.length > 0">
                    <p class="plan-title plan-danger">
                        Deleted resources ({{ plan.deleted.length }})
                    </p>
                    <table class="plan-table">
                        <tbody>
                            <tr
                                v-for="entry in plan.deleted"
                                :key="entry.resource_type + entry.uuid + entry.name"
                            >
                                <td>{{ entry.resource_type }}</td>
                                <td>{{ entry.name }}</td>
                                <td>{{ entry.uuid }}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <div v-if="plan.overwritten.length > 0">
                    <p class="plan-title">
                        Overwritten resources ({{ plan.overwritten.length }})
                    </p>
                    <table class="plan-table">
                        <tbody>
                            <tr
                                v-for="entry in plan.overwritten"
                                :key="entry.resource_type + entry.uuid + entry.name"
                            >
                                <td>{{ entry.resource_type }}</td>
                                <td>{{ entry.name }}</td>
                                <td>{{ entry.uuid }}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <div v-if="plan.id_map.length > 0">
                    <p class="plan-title">
                        Resources with a new UUID ({{ plan.id_map.length }})
                    </p>
                    <table class="plan-table">
                        <tbody>
                            <tr v-for="entry in plan.id_map" :key="entry.old_uuid">
                                <td>{{ entry.resource_type }}</td>
                                <td>{{ entry.old_uuid }}</td>
                                <td>→ {{ entry.new_uuid }}</td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <div v-if="plan.quota.length > 0">
                    <p class="plan-title">Quota after the restore</p>
                    <table class="plan-table">
                        <tbody>
                            <tr
                                v-for="usage in plan.quota"
                                :key="usage.resource_type"
                                :class="{ 'plan-danger': isExceeded(usage) }"
                            >
                                <td>{{ usage.resource_type }}</td>
                                <td>
                                    {{ usage.current }} →
                                    {{ usage.after_restore }} of {{ usage.max }}
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>

                <p class="hint-msg">
                    The project is locked during the restore. If the restore
                    fails, the project stays locked, until the restore is
                    repeated with force or an admin unlocks it.
                </p>
            </div>

            <div class="modal-bottombar">
                <div class="modal-actions">
                    <button
                        class="icon-button"
                        :disabled="submitting"
                        :title="plan ? 'Restore the backup' : 'Calculate the plan'"
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
import { ref, reactive, computed, watch } from "vue";

import { isekai } from "@/api";
import type {
    BackupResp,
    BackupRestoreReq,
    JobResp,
    QuotaUsageResp,
    RestorePlanResp,
} from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    backup: BackupResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept", job: JobResp): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    passphrase: "",
    targetProjectId: "",
    force: false,
    prune: false,
    startVms: false,
    includeDependencies: true,
});
const plan = ref<RestorePlanResp | null>(null);
const passphraseError = ref(false);
const submitting = ref(false);
const errorPopupMsg = ref<string>("");

const isProjectBackup = computed(() => props.backup?.kind === "PROJECT");

// prune is only allowed together with force
watch(
    () => form.force,
    (force) => {
        if (!force) form.prune = false;
    },
);

/** Same check like `QuotaUsageResp::is_exceeded` of the backend. */
function isExceeded(usage: QuotaUsageResp): boolean {
    return usage.after_restore > usage.max && usage.after_restore > usage.current;
}

function request(dryRun: boolean): BackupRestoreReq {
    return {
        passphrase: form.passphrase,
        force: form.force,
        prune: form.prune,
        target_project_id: form.targetProjectId.trim() || undefined,
        start_vms: form.startVms,
        include_dependencies: form.includeDependencies,
        dry_run: dryRun,
    };
}

async function handleAccept() {
    if (!props.backup) return;
    passphraseError.value = form.passphrase.length === 0;
    if (passphraseError.value) return;

    submitting.value = true;
    try {
        if (!plan.value) {
            // first step: only calculate the plan
            const result = await isekai.restoreBackup(props.backup.uuid, request(true));
            plan.value = result.plan;
        } else {
            // second step: restore with the same options, which were shown in the plan
            const result = await isekai.restoreBackup(props.backup.uuid, request(false));
            if (result.job) {
                emit("accept", result.job);
            }
        }
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            plan.value ? "Failed to restore backup" : "Failed to calculate the restore",
        );
    } finally {
        submitting.value = false;
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.backup-restore-modal {
    width: 46rem;
    max-height: 90vh;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

/* the back-button is made for the top of a card, within the modal it has no offset */
.plan-back-button {
    margin-top: 0;
}

.plan-title {
    font-weight: 600;
    margin-bottom: 0.3rem;
}

.plan-table {
    width: 100%;
    font-size: 0.85rem;
}

.plan-table td {
    padding: 0.15rem 0.5rem 0.15rem 0;
}

.plan-danger {
    color: var(--color-error);
}

.plan-warnings {
    font-size: 0.85rem;
}
</style>
