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
        <div class="modal user-passphrase-modal">
            <!-- Modal topbar -->
            <div class="modal-topbar">
                <span>Change passphrase</span>
            </div>

            <!-- Modal content -->
            <div class="modal-content">
                <strong>User: {{ user?.id }}</strong>
                <br />
                <br />
                <div>
                    <div>
                        <input
                            v-model="form.password"
                            type="password"
                            placeholder="New password"
                            :class="{ invalid_input: passwordError }"
                        />
                        <p v-if="passwordError" class="error-msg">
                            Password must be at least 8 characters
                        </p>
                    </div>
                    <br />
                    <div>
                        <input
                            v-model="form.confirmPassword"
                            type="password"
                            placeholder="Confirm new password"
                            :class="{ invalid_input: passwordConfirmError }"
                        />
                        <p v-if="passwordConfirmError" class="error-msg">
                            Password did not match
                        </p>
                    </div>
                </div>
            </div>

            <!-- Modal bottombar -->
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
import { ref, reactive, computed } from "vue";

import { miko } from "@/api";
import type { UserBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    user: UserBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "accept"): void;
    (e: "cancel"): void;
}>();

const form = reactive({
    password: "",
    confirmPassword: "",
});

const errorPopupMsg = ref<string>("");
const passwordError = ref(false);
const passwordConfirmError = computed(() =>
    form.password !== form.confirmPassword ? true : false,
);

/**
 * Sets the new passphrase for the selected user, which doesn't require the old passphrase.
 */
async function handleAccept() {
    if (!props.user) return;
    passwordError.value = form.password.length < 8;

    if (passwordError.value || passwordConfirmError.value) {
        return;
    }
    try {
        await miko.changePassphraseAdmin({
            user_id: props.user.id,
            new_passphrase: form.password,
        });

        emit("accept");
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to change passphrase",
        );
    }
}

function cancel() {
    emit("cancel");
}
</script>

<style scoped>
.user-passphrase-modal {
    width: 30em;
    margin-bottom: 5rem;
}
</style>
