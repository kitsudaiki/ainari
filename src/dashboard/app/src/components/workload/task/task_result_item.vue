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

<template>
    <div class="name-item">
        <label>{{ itemName }}</label>
    </div>
</template>

<script setup lang="ts">
interface Props {
    itemName: string;
}
const props = defineProps<Props>();

function getData() {
    return {
        hexagon: props.itemName,
    };
}
defineExpose({
    getData,
});
</script>

<style scoped>
.name-item {
    padding: 8px;
    border-bottom: 1px solid #ddd;
}
</style>
