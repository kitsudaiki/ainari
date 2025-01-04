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
#include <hanami_root.h>

/**
 * @brief search for an empty target-connection within a target-hexagon
 *
 * @param targetHexagon target-hexagon where to search
 * @param synapseBlockBuffer synapse-block-buffer to allocate new block,
 *                           if search-process was successful
 *
 * @return found empty connection, if seccessfule, else nullptr
 */
inline Connection*
searchTargetInHexagon(Hexagon* targetHexagon, ItemBuffer<SynapseBlock>& synapseBlockBuffer)
{
    const uint64_t numberOfConnectionsBlocks = targetHexagon->synapseBlockLinks.size();
    if (numberOfConnectionsBlocks == 0) {
        return nullptr;
    }

    const uint64_t synapseSectionPos
        = targetHexagon->synapseBlockLinks[rand() % numberOfConnectionsBlocks];
    if (synapseSectionPos == UNINIT_STATE_64) {
        return nullptr;
    }

    SynapseBlock* synapseBlocks = getItemData<SynapseBlock>(synapseBlockBuffer);
    Connection* connections = &synapseBlocks[synapseSectionPos].connections[0];
    if (connections[NUMBER_OF_SYNAPSESECTION - 1].active == true) {
        return nullptr;
    }

    return &connections[NUMBER_OF_SYNAPSESECTION - 1];
}

/**
 * @brief resize the number of blocks of a hexagon
 *
 * @param targetHexagon hexagon to resize
 */
inline void
resizeBlocks(Hexagon* targetHexagon, ItemBuffer<SynapseBlock>* synapseBlockBuffer)
{
    SynapseBlock block;
    const uint64_t synapseSectionPos = synapseBlockBuffer->addNewItem(block);
    if (synapseSectionPos == UNINIT_STATE_64) {
        return;
    }

    targetHexagon->header.numberOfBlocks++;

    // resize list
    targetHexagon->synapseBlockLinks.resize(targetHexagon->header.numberOfBlocks);
    targetHexagon->neuronBlocks.resize(targetHexagon->header.numberOfBlocks);
    targetHexagon->cluster->metrics.numberOfBlocks++;

    LOG_DEBUG("resized blocks to: " + std::to_string(targetHexagon->header.numberOfBlocks));

    // update content of list for the new size
    targetHexagon->synapseBlockLinks[targetHexagon->header.numberOfBlocks - 1] = synapseSectionPos;
    targetHexagon->neuronBlocks[targetHexagon->header.numberOfBlocks - 1] = NeuronBlock();

    targetHexagon->header.numberOfFreeSections += NUMBER_OF_SYNAPSESECTION;
}

/**
 * @brief allocate a new synapse-section
 *
 * @param cluster cluster to update
 * @param originLocation position of the soruce-neuron, which require the resize
 * @param lowerBound action-offset of the new section
 * @param potentialRange range of the potential, suppored by the section
 * @param synapseBlockBuffer synapse-block-buffer to allocate new blocks, if necessary
 *
 * @return true, if successful, else false
 */
inline bool
splitSection(Cluster& cluster, Connection* sourceConnection)
{
    // get origin object
    SourceLocation sourceLoc = getSourceNeuron(sourceConnection->origin, &cluster.hexagons[0]);
    if (sourceLoc.hexagon->header.isOutputHexagon) {
        return false;
    }

    // get target objects
    const uint32_t targetHexagonId
        = sourceLoc.hexagon->possibleHexagonTargetIds[rand() % NUMBER_OF_POSSIBLE_NEXT];
    if (targetHexagonId == UNINIT_STATE_32) {
        return false;
    }
    Hexagon* targetHexagon = &cluster.hexagons[targetHexagonId];
    ItemBuffer<SynapseBlock>* synapseBlockBuffer = &targetHexagon->attachedHost->synapseBlocks;

    // get target-connection
    Connection* targetConnection = searchTargetInHexagon(targetHexagon, *synapseBlockBuffer);
    if (targetConnection == nullptr) {
        return false;
    }
    targetHexagon->header.numberOfFreeSections--;
    targetHexagon->wasResized = true;
    cluster.metrics.numberOfSections++;

    // initialize new connection
    targetConnection->active = true;
    targetConnection->origin = sourceConnection->origin;
    targetConnection->lowerBound = sourceConnection->lowerBound + sourceConnection->splitValue;
    targetConnection->potentialRange
        = sourceConnection->potentialRange - sourceConnection->splitValue;
    sourceConnection->potentialRange = sourceConnection->splitValue;
    sourceConnection->splitValue = 0.0f;

    return true;
}

/**
 * @brief createNewSection
 * @param cluster
 * @param hexagon
 * @param neuron
 * @param blockId
 * @param neuronId
 * @return
 */
inline bool
createNewSection(Cluster& cluster,
                 Hexagon* hexagon,
                 Neuron* neuron,
                 const uint16_t blockId,
                 const uint8_t neuronId)
{
    ItemBuffer<SynapseBlock>* synapseBlockBuffer = &hexagon->attachedHost->synapseBlocks;

    // get target objects
    const uint32_t targetHexagonId
        = hexagon->possibleHexagonTargetIds[rand() % NUMBER_OF_POSSIBLE_NEXT];
    if (targetHexagonId == UNINIT_STATE_32) {
        return false;
    }
    Hexagon* targetHexagon = &cluster.hexagons[targetHexagonId];

    // get target-connection
    if (targetHexagon->header.numberOfFreeSections < NUMBER_OF_SYNAPSESECTION / 2) {
        resizeBlocks(targetHexagon, synapseBlockBuffer);
    }
    Connection* targetConnection = searchTargetInHexagon(targetHexagon, *synapseBlockBuffer);
    if (targetConnection == nullptr) {
        return false;
    }
    targetHexagon->header.numberOfFreeSections--;
    targetHexagon->wasResized = true;
    cluster.metrics.numberOfSections++;

    // initialize new connection
    targetConnection->active = true;
    targetConnection->origin.blockId = blockId;
    targetConnection->origin.neuronId = neuronId;
    targetConnection->origin.hexagonId = hexagon->header.hexagonId;
    targetConnection->origin.isInput = hexagon->inputInterface != nullptr;
    if (hexagon->header.isBinaryInput) {
        targetConnection->origin.isInput = 2;
    }
    targetConnection->lowerBound = 0.0f;
    targetConnection->potentialRange = std::numeric_limits<float>::max();

    neuron->inUse = 1;

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
updateCluster(Cluster& cluster, Hexagon* hexagon)
{
    ItemBuffer<SynapseBlock>* synapseBlockBuffer = &hexagon->attachedHost->synapseBlocks;
    SynapseBlock* synapseBlocks = getItemData<SynapseBlock>(*synapseBlockBuffer);

    Connection* connections = nullptr;
    Connection* connection = nullptr;
    bool found = false;
    uint64_t blockId = 0;
    uint8_t sourceId = 0;
    uint64_t link = 0;

    for (blockId = 0; blockId < hexagon->synapseBlockLinks.size(); ++blockId) {
        link = hexagon->synapseBlockLinks[blockId];
        connections = &synapseBlocks[link].connections[0];

        for (sourceId = 0; sourceId < NEURONS_PER_NEURONBLOCK; ++sourceId) {
            connection = &connections[sourceId];

            if (connection->splitValue > 0.0f) {
                if (splitSection(cluster, connection)) {
                    found = true;
                    connection->splitValue = 0.0f;
                }
            }
        }

        // resize if necessary
        // IMPORTANT: this must be done at the end, because the resize change the target of the
        // pointer
        if (hexagon->header.numberOfFreeSections < NUMBER_OF_SYNAPSESECTION / 2) {
            resizeBlocks(hexagon, synapseBlockBuffer);
        }
    }
    return found;
}

#endif  // HANAMI_SECTION_UPDATE_H
