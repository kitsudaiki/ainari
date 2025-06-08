/**
 * @file        cuda_host.cpp
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

#include "cuda_host.h"

#include <processing/cluster_resize.h>
#include <processing/cuda/cuda_functions.h>
#include <processing/cuda/cuda_worker_thread.h>

/**
 * @brief constructor
 *
 * @param localId identifier starting with 0 within the physical host and with the type of host
 */
CudaHost::CudaHost(const uint32_t localId, const GpuInfo& gpuInfo) : LogicalHost(localId)
{
    m_hostType = CUDA_HOST_TYPE;
    m_gpuInfo = gpuInfo;

    initBuffer();
    initWorkerThreads();
}

/**
 * @brief destructor
 */
CudaHost::~CudaHost() {}

/**
 * @brief initialize synpase-block-buffer based on the avaialble size of memory
 *
 * @param id local device-id
 */
void
CudaHost::initBuffer()
{
    const std::lock_guard<std::mutex> lock(cudaMutex);

    // m_totalMemory = getAvailableMemory_CUDA(id);
    // const uint64_t usedMemory = (m_totalMemory / 100) * 80;  // use 80% for synapse-blocks
    // blocks.initBuffer(usedMemory / sizeof(Block));
    // blocks.deleteAll();
    // Block* cpuBlocks = Hanami::getItemData<Block>(blocks);

    // deviceBlocks = initDevice_CUDA(cpuBlocks,
    // blocks.metaData.numberOfItems);
}

/**
 * @brief move the data of a cluster to this host
 *
 * @param cluster cluster to move
 *
 * @return true, if successful, else false
 */
bool
CudaHost::moveHexagon(Hexagon* hexagon)
{
    const std::lock_guard<std::mutex> lock(cudaMutex);

    // sync data from gpu to host, in order to have a consistent state

    LogicalHost* originHost = hexagon->attachedHost;
    Block* cpuBlocks = Hanami::getItemData<Block>(originHost->blocks);
    Block tempBlock;

    // copy synapse-blocks from the old host to this one
    for (uint64_t i = 0; i < hexagon->blockLinks.size(); ++i) {
        const uint64_t link = hexagon->blockLinks[i];
        if (link != UNINIT_STATE_64) {
            tempBlock = cpuBlocks[link];
            originHost->blocks.deleteItem(link);
            const uint64_t newPos = blocks.addNewItem(tempBlock);
            // TODO: make roll-back possible in error-case
            if (newPos == UNINIT_STATE_64) {
                return false;
            }
            hexagon->blockLinks[i] = newPos;
        }
    }

    // update data on gpu
    // hexagon->cudaPointer.deviceId = m_localId;
    // initHexagonOnDevice_CUDA(hexagon,
    //                          &hexagon->cluster->clusterHeader.settings,
    //                          getItemData<Block>(blocks),
    //                          deviceBlocks);

    hexagon->attachedHost = this;

    return true;
}
/**
 * @brief sync data of a cluster from gpu to host
 *
 * @param cluster cluster to sync
 */
void
CudaHost::syncWithHost(Hexagon* hexagon)
{
    const std::lock_guard<std::mutex> lock(cudaMutex);

    // Block* hostBlocks = getItemData<Block>(blocks);
    // copyFromGpu_CUDA(hexagon, hostBlocks, deviceBlocks);
}

/**
 * @brief remove the cluster-data from this host
 *
 * @param cluster cluster to remove
 */
void
CudaHost::removeHexagon(Hexagon* hexagon)
{
    const std::lock_guard<std::mutex> lock(cudaMutex);

    // remove synapse-blocks
    for (uint64_t& link : hexagon->blockLinks) {
        if (link != UNINIT_STATE_64) {
            blocks.deleteItem(link);
        }
    }

    // remove other data of the cluster, which are no synapse-blocks, from gpu
    // removeFromDevice_CUDA(hexagon);
}

bool
CudaHost::initWorkerThreads()
{
    CudaWorkerThread* newUnit = new CudaWorkerThread(this);
    m_workerThreads.push_back(newUnit);
    newUnit->startThread();
    newUnit->bindThreadToCore(0);

    return true;
}
