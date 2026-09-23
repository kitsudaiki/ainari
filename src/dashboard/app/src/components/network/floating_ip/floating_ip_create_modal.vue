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
        <div class="modal floating-ip-create-modal">
            <div class="modal-topbar">
                <span>Create floating IP</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.name"
                        type="text"
                        placeholder="Name"
                        :class="{ invalid_input: nameError }"
                    />
                    <p v-if="nameError" class="error-msg">
                        Name must be at least 4 characters
                    </p>
                </div>
                <br />
                <div class="field-row">
                    <label for="network">Network: </label>
                    <select
                        id="network"
                        v-model="form.network_uuid"
                        class="select-dropdown"
                        :class="{ invalid_input: networkError }"
                    >
                        <option value="" disabled>Select a network</option>
                        <option
                            v-for="network in networks"
                            :key="network.uuid"
                            :value="network.uuid"
                        >
                            {{ network.name }} ({{ network.subnet }})
                        </option>
                    </select>
                </div>
                <p v-if="networkError" class="error-msg">
                    A network must be selected
                </p>
                <br />
                <div>
                    <input
                        v-model="form.internal_ip"
                        type="text"
                        placeholder="Internal IP (for example 10.0.0.5)"
                        :class="{ invalid_input: internalIpError }"
                    />
                    <p v-if="internalIpError" class="error-msg">
                        Internal IP must be a valid IPv4-address
                    </p>
                </div>
                <br />
                <div>
                    <input
                        v-model="form.floating_ip"
                        type="text"
                        placeholder="Floating IP (optional)"
                        :class="{ invalid_input: floatingIpError }"
                    />
                    <p v-if="floatingIpError" class="error-msg">
                        Floating IP must be a valid IPv4-address
                    </p>
                    <p class="hint-msg">
                        Leave empty to let the backend pick a free address.
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
import { ref, reactive, onMounted } from "vue";

import { hanami } from "@/api";
import type { FloatingIpCreateReq, NetworkBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    icons: { acceptIcon: string; cancelIcon: string };
}
defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const nameError = ref(false);
const networkError = ref(false);
const internalIpError = ref(false);
const floatingIpError = ref(false);

const networks = ref<NetworkBasicResp[]>([]);

const form = reactive({
    name: "",
    network_uuid: "",
    internal_ip: "",
    floating_ip: "",
});

// plain IPv4-address. The backend validates this again, this check only avoids the
// obvious typos.
const IPV4_PATTERN = /^(\d{1,3}\.){3}\d{1,3}$/;

async function fetchNetworks() {
    try {
        networks.value = await hanami.listNetworks();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load networks");
    }
}

async function handleAccept() {
    nameError.value = form.name.length < 4;
    networkError.value = form.network_uuid === "";
    internalIpError.value = !IPV4_PATTERN.test(form.internal_ip);
    // the floating ip is optional, so it is only checked when something was typed in
    floatingIpError.value =
        form.floating_ip !== "" && !IPV4_PATTERN.test(form.floating_ip);

    if (
        nameError.value ||
        networkError.value ||
        internalIpError.value ||
        floatingIpError.value
    ) {
        return;
    }

    try {
        const body: FloatingIpCreateReq = {
            name: form.name,
            network_uuid: form.network_uuid,
            internal_ip: form.internal_ip,
        };
        if (form.floating_ip !== "") {
            body.floating_ip = form.floating_ip;
        }

        await hanami.createFloatingIp(body);

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to create floating IP",
        );
    }
}

function cancel() {
    emit("cancel");
}

onMounted(fetchNetworks);
</script>

<style scoped>
.floating-ip-create-modal {
    width: 32rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

.field-row {
    display: grid;
    grid-template-columns: 10rem 18rem; /* fixed input width */
    align-items: center;
}
</style>
