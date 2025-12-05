// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import axios from "axios";

const miko_api = axios.create({
    baseURL: import.meta.env.VITE_API_URL_MIKO,
});

const sakura_api = axios.create({
    baseURL: import.meta.env.VITE_API_URL_SAKURA,
});

export default {
    miko_api,
    sakura_api,
};
