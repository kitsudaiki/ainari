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
        <div class="modal user-info-modal">
            <div class="modal-topbar">
                <span>Info</span>
            </div>
            <div class="modal-content">
                <table>
                    <tbody>
                        <tr>
                            <td>ID</td>
                            <td>{{ user_info.id }}</td>
                        </tr>
                        <tr>
                            <td>Name</td>
                            <td>{{ user_info.name }}</td>
                        </tr>
                        <tr>
                            <td>Is Admin</td>
                            <td>
                                <div class="bool-icon">
                                    <img
                                        v-if="user_info.is_admin"
                                        :src="icons.acceptIcon"
                                        alt="True"
                                    />
                                    <img
                                        v-else
                                        :src="icons.cancelIcon"
                                        alt="False"
                                    />
                                </div>
                            </td>
                        </tr>
                        <tr>
                            <td>Created At</td>
                            <td>{{ user_info.created_at }}</td>
                        </tr>
                        <tr>
                            <td>Created By</td>
                            <td>{{ user_info.created_by }}</td>
                        </tr>
                        <tr>
                            <td>Updated At</td>
                            <td>{{ user_info.updated_at }}</td>
                        </tr>
                        <tr>
                            <td>Updated By</td>
                            <td>{{ user_info.updated_by }}</td>
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
import axios from "axios";

import { getAuthContext } from "@/auth_context";
import common from "@/common";
import { handleAxiosError } from "@/handleAxiosError";

const user_info = ref<{}[]>([]);
const errorPopupMsg = ref<string>("");

interface Props {
    user: { id: number; name: string } | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

async function fetchUserInfo(userId: string) {
    try {
        const authContext = getAuthContext();
        const miko_api = axios.create({
            baseURL: authContext.miko_address,
        });

        const response = await miko_api.get(`/v1alpha/user/${userId}/admin`, {
            headers: { Authorization: `Bearer ${authContext.token}` },
        });
        user_info.value = response.data;
        user_info.value.created_at = common.formatDateTime(
            user_info.value.created_at,
        );
        user_info.value.updated_at = common.formatDateTime(
            user_info.value.updated_at,
        );
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load user-info");
    }
}

function cancel() {
    emit("cancel");
}

onMounted(() => {
    fetchUserInfo(props.user.id);
});
</script>

<style scoped>
.user-info-modal {
    height: 30rem;
    width: 40rem;
}
</style>
