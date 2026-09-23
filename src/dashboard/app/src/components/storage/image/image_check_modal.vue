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
        <div class="modal image-check-modal">
            <div class="modal-topbar">
                <span>Check image</span>
            </div>
            <div class="modal-content">
                <p class="hint">
                    Compares a column of
                    <b>{{ props.image?.name }}</b> with a column of another
                    image and returns the ratio of the matching rows.
                </p>
                <br />
                <div>
                    <select
                        v-model="form.image_column"
                        class="select-dropdown"
                        :class="{ invalid_input: imageColumnError }"
                    >
                        <option value="" disabled>
                            Column of this image
                        </option>
                        <option
                            v-for="column in imageColumns"
                            :key="column"
                            :value="column"
                        >
                            {{ column }}
                        </option>
                    </select>
                    <p v-if="imageColumnError" class="error-msg">
                        A column of this image must be selected
                    </p>
                </div>
                <br />
                <div>
                    <select
                        v-model="form.reference_uuid"
                        class="select-dropdown"
                        :class="{ invalid_input: referenceUuidError }"
                    >
                        <option value="" disabled>Reference image</option>
                        <option
                            v-for="image in referenceImages"
                            :key="image.uuid"
                            :value="image.uuid"
                        >
                            {{ image.name }}
                        </option>
                    </select>
                    <p v-if="referenceUuidError" class="error-msg">
                        A reference image must be selected
                    </p>
                </div>
                <br />
                <div>
                    <select
                        v-model="form.reference_column"
                        class="select-dropdown"
                        :class="{ invalid_input: referenceColumnError }"
                    >
                        <option value="" disabled>
                            Column of the reference image
                        </option>
                        <option
                            v-for="column in referenceColumns"
                            :key="column"
                            :value="column"
                        >
                            {{ column }}
                        </option>
                    </select>
                    <p v-if="referenceColumnError" class="error-msg">
                        A column of the reference image must be selected
                    </p>
                </div>

                <template v-if="accuracy !== null">
                    <br />
                    <div class="divider">
                        <span>RESULT</span>
                    </div>
                    <p class="accuracy">
                        Accuracy: {{ (accuracy * 100).toFixed(2) }} %
                    </p>
                </template>
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
import { ref, reactive, computed, onMounted, watch } from "vue";

import { ryokan } from "@/api";
import type { ImageBasicResp } from "@/api";
import { handleAxiosError } from "@/handleAxiosError";

interface Props {
    image: ImageBasicResp | null;
    icons: { acceptIcon: string; cancelIcon: string };
}
const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "cancel"): void;
}>();

const errorPopupMsg = ref<string>("");
const imageColumnError = ref(false);
const referenceUuidError = ref(false);
const referenceColumnError = ref(false);

// the column-names are only provided by the get-endpoint, not by the list-endpoint,
// so they are requested separately for the image and for the selected reference
const imageColumns = ref<string[]>([]);
const referenceColumns = ref<string[]>([]);
const images = ref<ImageBasicResp[]>([]);

// result of the last check, `null` as long as no check was done
const accuracy = ref<number | null>(null);

const form = reactive({
    image_column: "",
    reference_uuid: "",
    reference_column: "",
});

/** An image can not be checked against itself, so it is left out of the list. */
const referenceImages = computed(() =>
    images.value.filter((entry) => entry.uuid !== props.image?.uuid),
);

async function fetchColumns(uuid: string): Promise<string[]> {
    try {
        const data = await ryokan.getImage(uuid);
        return data.column_names;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(
            err,
            "Failed to load columns of the image",
        );
        return [];
    }
}

async function handleAccept() {
    imageColumnError.value = form.image_column === "";
    referenceUuidError.value = form.reference_uuid === "";
    referenceColumnError.value = form.reference_column === "";

    if (
        imageColumnError.value ||
        referenceUuidError.value ||
        referenceColumnError.value
    ) {
        return;
    }

    if (!props.image) {
        return;
    }

    try {
        // the result is shown inside this modal instead of closing it, because
        // the check does not change anything, which the overview would have to
        // reload
        const result = await ryokan.checkImage(props.image.uuid, {
            image_column: form.image_column,
            reference_uuid: form.reference_uuid,
            reference_column: form.reference_column,
        });
        accuracy.value = result.accuracy;
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to check image");
    }
}

function cancel() {
    emit("cancel");
}

// reload the columns whenever another reference-image is selected and drop a
// column, which was picked for the previous one
watch(
    () => form.reference_uuid,
    async (uuid) => {
        form.reference_column = "";
        referenceColumns.value = uuid ? await fetchColumns(uuid) : [];
    },
);

onMounted(async () => {
    try {
        images.value = await ryokan.listImages();
    } catch (err) {
        errorPopupMsg.value = handleAxiosError(err, "Failed to load images");
    }

    if (props.image) {
        imageColumns.value = await fetchColumns(props.image.uuid);
    }
});
</script>

<style scoped>
.image-check-modal {
    width: 30rem;
}

.hint {
    font-size: 0.9rem;
}

.accuracy {
    font-size: 1.1rem;
}

/* is not found when I put this in one of the css files. Don't know why... */
.invalid_input {
    border-bottom: 2px solid #ff4d4f;
}
</style>
