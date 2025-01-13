/**
 * @file        cpu_host.cpp
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

#include "cpu_host.h"

#include <core/processing/cluster_resize.h>
#include <core/processing/cpu/core-backpropagation.h>
#include <core/processing/cpu/core-processing.h>
#include <core/processing/cpu/cpu_worker_thread.h>
#include <core/processing/cpu/output-backpropagation.h>
#include <core/processing/cpu/output-processing.h>
#include <core/processing/cpu/reduction.h>
#include <hanami_config/config_handler.h>
#include <hanami_cpu/memory.h>
#include <hanami_hardware/cpu_core.h>
#include <hanami_hardware/cpu_package.h>
#include <hanami_hardware/cpu_thread.h>
#include <hanami_hardware/host.h>

/**
 * @brief constructor
 *
 * @param localId identifier starting with 0 within the physical host and with the type of host
 */
CpuHost::CpuHost(const uint32_t localId) : LogicalHost(localId)
{
    m_hostType = CPU_HOST_TYPE;

    initBuffer();
    initWorkerThreads();
}

/**
 * @brief destructor
 */
CpuHost::~CpuHost() {}

/**
 * @brief initialize synpase-block-buffer based on the avaialble size of memory
 *
 * @param id local device-id
 */
void
CpuHost::initBuffer()
{
    m_totalMemory = getFreeMemory();
    bool success = false;
    float memoryUsage = GET_FLOAT_CONFIG("processing", "use_of_free_memory", success);
    // TODO: handle amound of min and max value by ranges inside of the config-lib
    if (memoryUsage < 0.1f) {
        memoryUsage = 0.1f;
    }
    if (memoryUsage > 0.9f) {
        memoryUsage = 0.9f;
    }
    const uint64_t usedMemory = static_cast<float>(m_totalMemory) * memoryUsage;
    blocks.initBuffer(usedMemory / sizeof(Block));
    blocks.deleteAll();

    LOG_INFO("Initialized number of syanpse-blocks on cpu-device: "
             + std::to_string(blocks.metaData.itemCapacity));
}

/**
 * @brief init processing-thread
 */
bool
CpuHost::initWorkerThreads()
{
    Host* host = Host::getInstance();
    CpuPackage* package = host->cpuPackages.at(m_localId);
    uint32_t threadCounter = 0;
    for (uint32_t coreId = 1; coreId < package->cpuCores.size(); coreId++) {
        for (uint32_t threadId = 0; threadId < package->cpuCores.at(coreId)->cpuThreads.size();
             threadId++)
        {
            CpuThread* thread = package->cpuCores.at(coreId)->cpuThreads.at(threadId);
            CpuWorkerThread* newUnit = new CpuWorkerThread(this);
            m_workerThreads.push_back(newUnit);
            newUnit->startThread();
            newUnit->bindThreadToCore(thread->threadId);
            threadCounter++;
        }
    }

    LOG_INFO("Initialized " + std::to_string(threadCounter) + " cpu worker-threads");

    return true;
}

/**
 * @brief move the data of a cluster to this host
 *
 * @param hexagon hexagon to move to the host
 *
 * @return true, if successful, else false
 */
bool
CpuHost::moveHexagon(Hexagon* hexagon)
{
    LogicalHost* sourceHost = hexagon->attachedHost;
    Block* targetBlocks = Hanami::getItemData<Block>(blocks);
    Block tempBlock;

    // copy synapse-blocks from the old host to this one here
    for (uint64_t pos = 0; pos < hexagon->blockLinks.size(); pos++) {
        const uint64_t synapseSectionPos = hexagon->blockLinks[pos];
        if (synapseSectionPos != UNINIT_STATE_64) {
            tempBlock = targetBlocks[synapseSectionPos];
            sourceHost->blocks.deleteItem(synapseSectionPos);
            const uint64_t newPos = blocks.addNewItem(tempBlock);
            // TODO: make roll-back possible in error-case
            if (newPos == UNINIT_STATE_64) {
                return false;
            }
            hexagon->blockLinks[pos] = newPos;
        }
    }

    hexagon->attachedHost = this;

    return true;
}

/**
 * @brief empty function in this case
 */
void
CpuHost::syncWithHost(Hexagon*)
{
    return;
}

/**
 * @brief remove synpase-blocks of a cluster from the host-buffer
 *
 * @param hexagon hexagon to remove from host
 */
void
CpuHost::removeHexagon(Hexagon* hexagon)
{
    for (uint64_t blockLink : hexagon->blockLinks) {
        if (blockLink != UNINIT_STATE_64) {
            blocks.deleteItem(blockLink);
        }
    }
}
