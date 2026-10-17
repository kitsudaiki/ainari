<!--
// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//         http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
-->

<!--
Bar, which shows the progress of a long running operation, like an upload. If the progress is
unknown, for example while the server processes an uploaded file, the bar is indeterminate and
shows a moving stripe instead.
-->
<template>
    <div
        class="progress-bar"
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-valuenow="indeterminate ? undefined : clampedPercentage"
        :aria-label="label"
    >
        <div
            class="progress-bar-fill"
            :class="{ indeterminate }"
            :style="indeterminate ? undefined : { width: clampedPercentage + '%' }"
        ></div>
        <span class="progress-bar-text">{{ label }}</span>
    </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Props {
    /** progress between 0 and 100 */
    percentage?: number;
    /** text, which is shown on top of the bar */
    label?: string;
    /** true, if the progress is unknown */
    indeterminate?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
    percentage: 0,
    label: "",
    indeterminate: false,
});

const clampedPercentage = computed(() =>
    Math.min(100, Math.max(0, Math.round(props.percentage))),
);
</script>

<style scoped>
.progress-bar {
    position: relative;
    width: 100%;
    height: 1.5rem;
    border: 1px solid var(--color-border-soft);
    overflow: hidden;
    background-color: rgba(255, 255, 255, 0.04);
}

.progress-bar-fill {
    height: 100%;
    background-color: var(--color-highlight);
    /* the fill is split into small segments with gaps, like a charging-bar */
    mask-image: repeating-linear-gradient(
        90deg,
        #000 0 6px,
        transparent 6px 8px
    );
    transition: width 0.2s ease;
}

.progress-bar-fill.indeterminate {
    width: 30%;
    animation: progress-bar-slide 1.4s ease-in-out infinite;
}

@keyframes progress-bar-slide {
    from {
        transform: translateX(-100%);
    }
    to {
        transform: translateX(340%);
    }
}

/* without animations the indeterminate bar is shown as full, dimmed bar */
@media (prefers-reduced-motion: reduce) {
    .progress-bar-fill.indeterminate {
        width: 100%;
        opacity: 0.5;
        animation: none;
    }
}

/* the shadow keeps the text readable on the empty and on the filled part of the bar */
.progress-bar-text {
    position: absolute;
    inset: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    font-weight: bold;
    white-space: nowrap;
    color: var(--color-text);
    text-shadow: 0 0 0.2rem #000000;
}

</style>
