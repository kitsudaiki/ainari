/**
 * @file        gpu_kernel.cu
 *
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
 *
 *      Licensed under the Apache License, Version 2.0 (the "License");
 *      you may not use this file except in compliance with the License.
 *      You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 *      Unless required by applicable law or agreed to in writing, software
 *      distributed under the License is distributed on an "AS IS" BASIS,
 *      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *      See the License for the specific language governing permissions and
 *      limitations under the License.
 */

// #include <cuda_runtime_api.h>
#include <stdint.h>

#include <iostream>
#include <nlohmann/json.hpp>
#include <string>

using json = nlohmann::json;

struct GpuInfo {
    uint64_t deviceId = 0;
    std::string name = "";
    uint64_t numberOfCudaCores = 0;
    uint64_t numberOfCoresPerSM = 0;
    uint64_t maxNumberOfThreadsPerBlock = 0;
    uint64_t sizeOfSharedMemPerBlock = 0;
    uint64_t totalMemory = 0;
    uint64_t freeMemory = 0;

    const json toJson() const
    {
        json result = {
            {"device_id", deviceId},
            {"name", name},
            {"number_of_cuda_cores", numberOfCudaCores},
            {"number_of_cores_per_sm", numberOfCoresPerSM},
            {"max_number_of_threads_per_block", maxNumberOfThreadsPerBlock},
            {"size_of_shared_mem:per_Block", sizeOfSharedMemPerBlock},
            {"total_memory", totalMemory},
            {"free_memory", freeMemory},
        };

        return result;
    }
};

void getGpuInfos(std::vector<GpuInfo>& results);
