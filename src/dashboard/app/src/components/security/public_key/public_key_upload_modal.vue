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
        <div class="modal public-key-upload-modal">
            <div class="modal-topbar">
                <span>Upload public key</span>
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
                <div>
                    <label>Public key (openssh one-line format):</label>
                    <textarea
                        id="public_key_input"
                        v-model="form.publicKey"
                        :class="{ invalid_input: publicKeyError }"
                    ></textarea>
                    <p v-if="publicKeyError" class="error-msg">
                        Public key must be a single openssh-line, for example
                        starting with ssh-ed25519 or ssh-rsa
                    </p>
                    <p class="hint-msg">
                        The fingerprint is calculated by the omamori.
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

import { omamori } from "@/api";
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
const publicKeyError = ref(false);

const form = reactive({
    name: "",
    publicKey: "",
});

// one-line openssh-representation, for example "ssh-ed25519 AAAA... comment". The
// omamori validates this again, this check only avoids the obvious mistakes like
// pasting a private key or a whole file.
const OPENSSH_PATTERN = /^(ssh|ecdsa)-[a-z0-9@.-]+\s+[A-Za-z0-9+/=]+(\s+\S+)?$/;

async function handleAccept() {
    const publicKey = form.publicKey.trim();

    nameError.value = form.name.length < 4;
    publicKeyError.value = !OPENSSH_PATTERN.test(publicKey);

    if (nameError.value || publicKeyError.value) {
        return;
    }

    try {
        await omamori.uploadPublicKey(form.name, publicKey);
        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to upload public key",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.public-key-upload-modal {
    min-width: 34rem;
}

#public_key_input {
    height: 8rem;
    width: 100%;
}

.hint-msg {
    font-size: 0.8rem;
    opacity: 0.7;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
