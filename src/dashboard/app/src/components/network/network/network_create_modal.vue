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
        <div class="modal network-create-modal">
            <div class="modal-topbar">
                <span>Create network</span>
            </div>
            <div class="modal-content">
                <div>
                    <input
                        v-model="form.name"
                        type="text"
                        placeholder="Network-Name"
                        :class="{ invalid_input: nameError }"
                    />
                    <p v-if="nameError" class="error-msg">
                        Network-Name must be at least 4 characters
                    </p>
                </div>
                <br />
                <div>
                    <input
                        v-model="form.subnet"
                        type="text"
                        placeholder="Subnet (for example 10.0.0.0/24)"
                        :class="{ invalid_input: subnetError }"
                    />
                    <p v-if="subnetError" class="error-msg">
                        Subnet must be given in CIDR-notation
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
import { ref, reactive } from "vue";

import { hanami } from "@/api";
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
const subnetError = ref(false);

const form = reactive({
    name: "",
    subnet: "",
});

// IPv4-subnet in CIDR-notation, for example 10.0.0.0/24. The backend validates this
// again, this check only avoids the obvious typos.
const SUBNET_PATTERN = /^(\d{1,3}\.){3}\d{1,3}\/\d{1,2}$/;

async function handleAccept() {
    nameError.value = form.name.length < 4;
    subnetError.value = !SUBNET_PATTERN.test(form.subnet);

    if (nameError.value || subnetError.value) {
        return;
    }

    try {
        await hanami.createNetwork({
            name: form.name,
            subnet: form.subnet,
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to create network");
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.network-create-modal {
    width: 30rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
