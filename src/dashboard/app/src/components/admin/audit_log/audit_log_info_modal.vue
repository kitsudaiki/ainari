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
        <div class="modal audit-log-info-modal">
            <div class="modal-topbar">
                <span>Info</span>
            </div>
            <div class="modal-content">
                <table v-if="entry_info">
                    <tbody>
                        <tr>
                            <td>UUID</td>
                            <td>{{ entry_info.uuid }}</td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ entry_info.created_at }}</td>
                        </tr>
                        <tr>
                            <td>Component</td>
                            <td>{{ entry_info.component_name }}</td>
                        </tr>
                        <tr>
                            <td>Host</td>
                            <td>{{ entry_info.host_name }}</td>
                        </tr>
                        <tr>
                            <td>User</td>
                            <td>{{ entry_info.user_id || "-" }}</td>
                        </tr>
                        <tr>
                            <td>Project</td>
                            <td>{{ entry_info.project_id || "-" }}</td>
                        </tr>
                        <tr>
                            <td>Request</td>
                            <td>{{ entry_info.request_type }}</td>
                        </tr>
                        <tr>
                            <td>Endpoint</td>
                            <td>{{ entry_info.endpoint }}</td>
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

import { omikuji } from "@/api";
import type { AuditLogResp } from "@/api";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

const entry_info = ref<AuditLogResp | null>(null);
const errorPopupMsg = ref<string>("");

interface Props {
    // only the uuid is needed, because the entry is loaded by the modal itself
    entry: Pick<AuditLogResp, "uuid"> | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();

async function fetchEntryInfo(uuid: string) {
    try {
        const data = await omikuji.getAuditLog(uuid);
        data.created_at = common.formatDateTime(data.created_at);
        entry_info.value = data;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load the audit-log-entry",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    if (props.entry) {
        fetchEntryInfo(props.entry.uuid);
    }
});
</script>

<style scoped>
.audit-log-info-modal {
    width: 60rem;
}

/* the values can be long, like a message or an endpoint with query-parameters */
.audit-log-info-modal td:last-child {
    white-space: pre-wrap;
    word-break: break-word;
}
</style>
