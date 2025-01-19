/**
 * @file        backpropagation.h
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

#ifndef HANAMI_CORE_BACKPROPAGATION_H
#define HANAMI_CORE_BACKPROPAGATION_H

#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cpu/cpu_host.h>
#include <core/processing/logical_host.h>
#include <hanami_root.h>
#include <math.h>

#include <cmath>

/**
 * @brief backpropagate all neurons, which are not connected to
 *        an output-interface
 *
 * @param hexagon pointer to current hexagon
 * @param blockId id of the current block within the hexagon
 */
inline void
_backpropagateNeuron(Hexagon* hexagon, const uint32_t blockId)
{
    Axon* axon = nullptr;
    AxonBlock* axonBlock = &hexagon->axonBlocks[blockId];

    for (uint8_t neuronId = 0; neuronId < NEURONS_PER_BLOCK; ++neuronId) {
        axon = &axonBlock->axons[neuronId];

        if (axon->potential < POTENTIAL_BORDER) {
            continue;
        }
        axon->delta *= 1.4427f * pow(0.5f, axon->potential);
    }
}

/**
 * @brief backpropagate all exit-neurons, which are connected to
 *        an output-interface
 *
 * @param hexagon pointer to current hexagon
 * @param blockId id of the current block within the hexagon
 */
inline void
_backpropagateExitNeuron(Hexagon* hexagon, const uint32_t blockId)
{
    Axon* axon = nullptr;
    AxonBlock* axonBlock = &hexagon->axonBlocks[blockId];

    for (uint8_t neuronId = 0; neuronId < NEURONS_PER_BLOCK; ++neuronId) {
        axon = &axonBlock->axons[neuronId];
        axon->delta *= axon->potential * (1 - axon->potential);
    }
}

/**
 * @brief backpropagate a synapse-section and adjust weights
 *
 * @param section current synapse-section
 * @param connection current connection related to the synapse-section
 * @param targetTempBlock temp-value-block of the target neuron-block
 * @param axon source-axon, which triggered the section
 */
inline void
_backpropagateSection(SynapseSection* section,
                      Connection* connection,
                      AxonBlock* targetBlock,
                      Axon* axon)
{
    float potential = axon->potential - connection->lowerBound;
    uint8_t pos = 0;
    Synapse* synapse;
    Axon* targetAxon = nullptr;
    constexpr float trainValue = 0.1f;
    float delta = 0.0f;

    // iterate over all synapses in the section
    while (pos < SYNAPSES_PER_SECTION && potential > POTENTIAL_BORDER) {
        synapse = &section->synapses[pos];

        targetAxon = &targetBlock->axons[synapse->targetNeuronId % NEURONS_PER_BLOCK];
        delta = targetAxon->delta * synapse->weight1;
        delta += targetAxon->delta * synapse->weight2;
        synapse->weight1 -= trainValue * targetAxon->delta;
        synapse->weight2 -= trainValue * targetAxon->delta;
        axon->delta += delta;

        potential -= synapse->border;
        ++pos;
    }
}

/**
 * @brief backpropagate block
 *
 * @param hexagon pointer to current hexagon
 * @param blocks pointer to synapse-blocks
 * @param blockId id of the current block within the hexagon
 */
inline void
_backpropagateBlock(Hexagon* hexagon, Block* blocks, const uint32_t blockId)
{
    Connection* connection = nullptr;
    AxonBlock* axonBlock = nullptr;
    SynapseSection* synapseSection = nullptr;
    Block* block = nullptr;
    AxonBlock* tansferAxonBlocks = &hexagon->transferAxonBlocks[0];
    Axon* axon = nullptr;

    if (blockId >= hexagon->header.numberOfBlocks) {
        return;
    }
    const uint64_t blockLink = hexagon->blockLinks[blockId];

    axonBlock = &hexagon->axonBlocks[blockId];
    block = &blocks[blockLink];

    for (uint32_t i = 0; i < NUMBER_OF_SECTIONS - 1; ++i) {
        connection = &block->connections[i];
        axon = &tansferAxonBlocks[connection->sourceBlockId].axons[connection->sourceId];

        if (connection->active == true && axon->potential > POTENTIAL_BORDER) {
            synapseSection = &block->sections[i];
            _backpropagateSection(synapseSection, connection, axonBlock, axon);
        }
    }
}

/**
 * @brief run the backpropagation over the core the cluster
 *
 * @param cluster pointer to cluster
 * @param hexagonId id of the hexagon to process
 * @param blockId id of the block within the hexagon
 */
inline void
backpropagateBlock(Cluster* cluster, const uint32_t hexagonId, const uint32_t blockId)
{
    Hanami::ErrorContainer error;
    Hexagon* hexagon = &cluster->hexagons[hexagonId];
    Block* blocks = getItemData<Block>(hexagon->attachedHost->blocks);

    if (hexagon->outputInterface == nullptr) {
        _backpropagateNeuron(hexagon, blockId);
    }
    else {
        _backpropagateExitNeuron(hexagon, blockId);
    }

    _backpropagateBlock(hexagon, blocks, blockId);
}

/**
 * @brief send one transfer-axon-block back to their source
 *
 * @param cluster pointer to cluster
 * @param axonBlock block to transfer
 */
inline void
_transferAxonBlocksBackwards(Cluster* cluster, AxonBlock* axonBlock)
{
    if (axonBlock->sourceBlockId == UNINIT_STATE_32
        || axonBlock->sourceHexagonId == UNINIT_STATE_32)
    {
        return;
    }

    Hexagon* sourceHexagon = &cluster->hexagons[axonBlock->sourceHexagonId];
    sourceHexagon->axonBlocks[axonBlock->sourceBlockId] = *axonBlock;
}

/**
 * @brief send axon-blocks of an output-interface back to the hexagon
 *
 * @param hexagon pointer to current processed hexagon
 */
inline void
transferAxonBlockFromOutput(Hexagon* hexagon)
{
    // std::cout << "transferAxonBlockFromOutput: x" << hexagon->header.hexagonId << std::endl;

    for (uint64_t blockId = 0; blockId < hexagon->axonBlocks.size(); ++blockId) {
        AxonBlock* axonBlock = &hexagon->axonBlocks[blockId];
        *axonBlock = hexagon->outputInterface->targetAxonBlocks[blockId];
    }
}

/**
 * @brief fake-transfer to the input just to finish the cycle properly
 *
 * @param hexagon pointer to current processed hexagon
 */
inline void
transferAxonBlockToInput(Hexagon* hexagon)
{
    for (uint64_t blockId = 0; blockId < hexagon->transferAxonBlocks.size(); ++blockId) {
        AxonBlock* axonBlock = &hexagon->transferAxonBlocks[blockId];
        hexagon->inputInterface->inputAxons[blockId] = *axonBlock;
    }
}

/**
 * @brief send transfer-axon-blocks back to their source
 *
 * @param cluster pointer to cluster
 * @param hexagon pointer to current processed hexagon
 */
inline void
transferAxonBlocksBackward(Cluster* cluster, Hexagon* hexagon)
{
    for (uint64_t blockId = 0; blockId < hexagon->transferAxonBlocks.size(); ++blockId) {
        AxonBlock* transferAxonBlock = &hexagon->transferAxonBlocks[blockId];
        _transferAxonBlocksBackwards(cluster, transferAxonBlock);
    }
}

#endif  // HANAMI_CORE_BACKPROPAGATION_H
