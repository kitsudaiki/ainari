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

#include "info.h"

#include <math.h>

#include <chrono>
#include <iostream>

uint64_t
getCudaCoresPerSM(const uint32_t major, const uint32_t minor)
{
    struct SMtoCores {
        int32_t SM;  // 0xMm (hexadecimal notation), M = SM Major version, and m = SM minor version
        int32_t Cores;
    };

    SMtoCores gpuArchCoresPerSM[] = {
        {0x30, 192},  // Kepler Generation (SM 3.0) GK10x class
        {0x32, 192},  // Kepler Generation (SM 3.2) GK10x class
        {0x35, 192},  // Kepler Generation (SM 3.5) GK11x class
        {0x37, 192},  // Kepler Generation (SM 3.7) GK21x class
        {0x50, 128},  // Maxwell Generation (SM 5.0) GM10x class
        {0x52, 128},  // Maxwell Generation (SM 5.2) GM20x class
        {0x53, 128},  // Maxwell Generation (SM 5.3) GM20x class
        {0x60, 64},   // Pascal Generation (SM 6.0) GP100 class
        {0x61, 128},  // Pascal Generation (SM 6.1) GP10x class
        {0x62, 128},  // Pascal Generation (SM 6.2) GP10x class
        {0x70, 64},   // Volta Generation (SM 7.0) GV100 class
        {0x72, 64},   // Volta Generation (SM 7.2) GV11b class
        {0x75, 64},   // Turing Generation (SM 7.5) TU10x class
        {0x80, 64},   // Ampere Generation (SM 8.0) GA100 class
        {0x86, 128},  // Ampere Generation (SM 8.6) GA10x class
        {0x87, 128},  // Ampere Generation (SM 8.7) GA10x class
        {-1, -1}      // End of list marker
    };

    for (int i = 0; gpuArchCoresPerSM[i].SM != -1; ++i) {
        if (gpuArchCoresPerSM[i].SM == ((major << 4) + minor)) {
            return gpuArchCoresPerSM[i].Cores;
        }
    }

    std::cerr << "Unknown SM version." << std::endl;
    return 0;
}

void
getGpuInfos(std::vector<GpuInfo>& results)
{
    //    int deviceCount;
    //    cudaError_t cudaResult = cudaGetDeviceCount(&deviceCount);
    //    if (cudaResult != cudaSuccess || deviceCount <= 0) {
    //        return;
    //    }

    //    for (uint64_t deviceId = 0; deviceId < deviceCount; deviceId++) {
    //        GpuInfo info;
    //        cudaSetDevice(deviceId);
    //        cudaMemGetInfo(&info.freeMemory, &info.totalMemory);

    //        cudaDeviceProp deviceProp;
    //        cudaGetDeviceProperties(&deviceProp, deviceId);
    //        info.deviceId = deviceId;
    //        info.name = std::string(deviceProp.name);
    //        if (info.name.size() > 256) {
    //            info.name = "";
    //        }
    //        info.numberOfCoresPerSM = getCudaCoresPerSM(deviceProp.major, deviceProp.minor);
    //        info.numberOfCudaCores = info.numberOfCoresPerSM * deviceProp.multiProcessorCount;
    //        info.maxNumberOfThreadsPerBlock = deviceProp.maxThreadsPerBlock;
    //        info.sizeOfSharedMemPerBlock = deviceProp.sharedMemPerBlock;

    //        results.push_back(info);
    //    }
}
