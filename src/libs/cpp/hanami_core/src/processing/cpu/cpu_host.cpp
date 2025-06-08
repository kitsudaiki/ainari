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

#include <processing/cluster_resize.h>
#include <processing/cpu/core_backpropagation.h>
#include <processing/cpu/core_processing.h>
#include <processing/cpu/cpu_worker_thread.h>
#include <processing/cpu/output_backpropagation.h>
#include <processing/cpu/output_processing.h>

/**
 * @brief constructor
 *
 * @param localId identifier starting with 0 within the physical host and with the type of host
 */
CpuHost::CpuHost(const uint32_t localId,
                 const uint64_t maxMemoryUsage,
                 const uint64_t numberOfThreads)
    : LogicalHost(localId)
{
    m_hostType = CPU_HOST_TYPE;

    initBuffer(maxMemoryUsage);
    initWorkerThreads(numberOfThreads);
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
CpuHost::initBuffer(const uint64_t maxMemoryUsage)
{
    // one block can have up to 512 entries, but considering, that they are not all fully filled,
    // 256 was choosen to estimate the average fill rate
    const uint64_t numberOfBlocks
        = maxMemoryUsage / (sizeof(Block) + (256 * sizeof(SynapseSection)));
    blocks.initBuffer(numberOfBlocks);
    blocks.deleteAll();

    sections.initBuffer(numberOfBlocks * 256);
    sections.deleteAll();
}

/**
 * @brief init processing-thread
 */
bool
CpuHost::initWorkerThreads(const uint64_t numberOfThreads)
{
    for (uint64_t i = 0; i < numberOfThreads; ++i) {
        CpuWorkerThread* newUnit = new CpuWorkerThread(this);
        m_workerThreads.push_back(newUnit);
        newUnit->startThread();
        newUnit->bindThreadToCore(i);
    }

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
    Block* blocks = Hanami::getItemData<Block>(hexagon->attachedHost->blocks);

    // delete sections from buffer
    for (uint64_t blockLink : hexagon->blockLinks) {
        Block* block = &blocks[blockLink];
        for (Connection& connection : block->connections) {
            if (connection.sectionPtr != UNINIT_STATE_64) {
                hexagon->attachedHost->blocks.deleteItem(connection.sectionPtr);
            }
        }
    }

    // delete blocks from buffer
    for (uint64_t blockLink : hexagon->blockLinks) {
        if (blockLink != UNINIT_STATE_64) {
            hexagon->attachedHost->blocks.deleteItem(blockLink);
        }
    }
}
