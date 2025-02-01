/**
 * @file        cluster_resize.h
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

#ifndef HANAMI_SECTION_UPDATE_H
#define HANAMI_SECTION_UPDATE_H

#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cpu/cpu_host.h>
#include <core/processing/cuda/cuda_host.h>
#include <core/processing/logical_host.h>
#include <core/processing/physical_host.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>

/**
 * @brief initialize a new synpase
 *
 * @param synapse pointer to the synapse, which should be (re-) initialized
 * @param remainingW new weight for the synapse
 * @param randomSeed reference to the current seed of the randomizer
 */
inline void
createNewSynapse(Synapse* synapse, const float remainingW, uint32_t& randomSeed)
{
    constexpr float randMax = static_cast<float>(RAND_MAX);
    constexpr float sigNeg = 0.5f;
    const uint32_t signRand = Hanami::pcg_hash(randomSeed) % 1000;

    synapse->border = remainingW;
    synapse->activeCounter = 5;
    synapse->targetNeuronId = Hanami::pcg_hash(randomSeed) % NEURONS_PER_BLOCK;
    synapse->weight1 = (static_cast<float>(Hanami::pcg_hash(randomSeed)) / randMax) / 10.0f;
    synapse->weight1 *= static_cast<float>(1.0f - (1000.0f * sigNeg > signRand) * 2);
    synapse->weight2 = (static_cast<float>(Hanami::pcg_hash(randomSeed)) / randMax) / 10.0f;
    synapse->weight2 *= static_cast<float>(1.0f - (1000.0f * sigNeg > signRand) * 2);
}

/**
 * @brief search for an empty target-connection within a target-hexagon
 *
 * @param targetHexagon target-hexagon where to search
 * @param blockBuffer synapse-block-buffer to allocate new block,
 *                           if search-process was successful
 *
 * @return found empty connection, if seccessfule, else nullptr
 */
inline Connection*
searchTargetInHexagon(Hexagon* hexagon, ItemBuffer<Block>& blockBuffer)
{
    // check if there is even free space
    const uint64_t numberOfConnectionsBlocks = hexagon->blockLinks.size();
    if (numberOfConnectionsBlocks == 0) {
        return nullptr;
    }

    // select a random target-block within the current hexagon
    const uint64_t targetBlockLink = hexagon->blockLinks[rand() % numberOfConnectionsBlocks];
    if (targetBlockLink == UNINIT_STATE_64) {
        return nullptr;
    }

    Block* blocks = getItemData<Block>(blockBuffer);
    Connection* connections = &blocks[targetBlockLink].connections[0];

    // search for free connection
    uint32_t foundLocation = UNINIT_STATE_32;
    for (uint32_t i = 0; i < NUMBER_OF_SECTIONS; ++i) {
        if (connections[i].active == false) {
            foundLocation = i;
            break;
        }
    }

    // check if something was found
    if (foundLocation == UNINIT_STATE_32) {
        return nullptr;
    }

    // initialize found entry
    SynapseSection* sections = &blocks[targetBlockLink].sections[0];
    uint32_t randomSeed = rand();
    createNewSynapse(&sections[foundLocation].synapses[0], 1.0f, randomSeed);

    return &connections[foundLocation];
}

/**
 * @brief resize the number of blocks of a hexagon
 *
 * @param targetHexagon hexagon to resize
 */
inline void
resizeBlocks(Hexagon* targetHexagon, ItemBuffer<Block>* blockBuffer)
{
    Block block;
    const uint64_t synapseSectionPos = blockBuffer->addNewItem(block);
    if (synapseSectionPos == UNINIT_STATE_64) {
        return;
    }

    targetHexagon->header.numberOfBlocks++;

    // resize list
    targetHexagon->blockLinks.resize(targetHexagon->header.numberOfBlocks);
    targetHexagon->axonBlocks.resize(targetHexagon->header.numberOfBlocks);
    targetHexagon->cluster->metrics.numberOfBlocks++;

    LOG_DEBUG("resized blocks to: " + std::to_string(targetHexagon->header.numberOfBlocks));

    // update content of list for the new size
    targetHexagon->blockLinks[targetHexagon->header.numberOfBlocks - 1] = synapseSectionPos;
    targetHexagon->axonBlocks[targetHexagon->header.numberOfBlocks - 1] = AxonBlock();

    targetHexagon->header.numberOfFreeSections += NUMBER_OF_SECTIONS;
}

/**
 * @brief allocate a new synapse-section
 *
 * @param cluster cluster to update
 * @param originLocation position of the soruce-neuron, which require the resize
 * @param lowerBound action-offset of the new section
 * @param potentialRange range of the potential, suppored by the section
 * @param blockBuffer synapse-block-buffer to allocate new blocks, if necessary
 *
 * @return true, if successful, else false
 */
inline bool
splitSection(Cluster* cluster,
             Hexagon* hexagon,
             Connection* sourceConnection,
             AxonBlock* sourceAxonBlocks)
{
    if (sourceConnection->sourceBlockId == UNINIT_STATE_32) {
        return false;
    }

    // get origin object
    Axon* sourceAxon
        = &sourceAxonBlocks[sourceConnection->sourceBlockId].axons[sourceConnection->sourceId];

    // get target objects
    ItemBuffer<Block>* blockBuffer = &hexagon->attachedHost->blocks;
    Connection* targetConnection = searchTargetInHexagon(hexagon, *blockBuffer);
    if (targetConnection == nullptr) {
        return false;
    }
    hexagon->header.numberOfFreeSections--;
    hexagon->wasResized = true;
    cluster->metrics.numberOfSections++;
    // std::cout<<"cluster->metrics.numberOfSections1:
    // "<<cluster->metrics.numberOfSections<<std::endl;
    // initialize new connection
    targetConnection->active = true;
    targetConnection->sourceBlockId = sourceConnection->sourceBlockId;
    targetConnection->sourceId = sourceConnection->sourceId;
    targetConnection->lowerBound = sourceConnection->lowerBound + sourceConnection->splitValue;
    targetConnection->potentialRange
        = sourceConnection->potentialRange - sourceConnection->splitValue;
    sourceConnection->potentialRange = sourceConnection->splitValue;
    sourceConnection->splitValue = 0.0f;

    return true;
}

/**
 * @brief iterate over all neuron and run the resize-process, if necessary. This function is used
 *        in case of a cuda host, where the resize has to be done after the processing
 *
 * @param cluster cluster to resize
 *
 * @return true, if a resize was performed, else false. This is used to avoid unnecessary data-
 *         transfers to the gpu
 */
inline bool
updateCluster(Cluster* cluster, Hexagon* hexagon)
{
    ItemBuffer<Block>* blockBuffer = &hexagon->attachedHost->blocks;
    Block* blocks = getItemData<Block>(*blockBuffer);

    Connection* connections = nullptr;
    Connection* connection = nullptr;
    bool found = false;
    uint64_t blockId = 0;
    uint8_t sourceId = 0;
    uint64_t link = 0;

    for (blockId = 0; blockId < hexagon->blockLinks.size(); ++blockId) {
        link = hexagon->blockLinks[blockId];
        connections = &blocks[link].connections[0];

        for (sourceId = 0; sourceId < NEURONS_PER_BLOCK; ++sourceId) {
            connection = &connections[sourceId];

            if (connection->splitValue > 0.0f) {
                if (splitSection(cluster, hexagon, connection, &hexagon->transferAxonBlocks[0])) {
                    found = true;
                    connection->splitValue = 0.0f;
                }
            }
        }
    }

    // resize if necessary
    // IMPORTANT: this must be done at the end, because the resize change the target of the
    // pointer
    if (hexagon->header.numberOfFreeSections < NUMBER_OF_SECTIONS / 2) {
        // std::cout << "++++++++++++++++++++++++++++++++++++ resize: " << hexagon->header.hexagonId
        //           << "  " << hexagon->blockLinks.size() << std::endl;
        resizeBlocks(hexagon, blockBuffer);
    }

    return found;
}

#endif  // HANAMI_SECTION_UPDATE_H
