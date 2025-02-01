/**
 * @file        processing.h
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

#ifndef HANAMI_CORE_PROCESSING_H
#define HANAMI_CORE_PROCESSING_H

#include <api/websocket/cluster_io.h>
#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/cluster_resize.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>
#include <math.h>

#include <cmath>

/**
 * @brief process a single synapse-section
 *
 * @param cluster cluster, where the synapseSection belongs to
 * @param synapseSection current synapse-section to process
 * @param connection pointer to the connection-object, which is related to the section
 * @param transferAxon pointer to source-axon, which triggered the section
 * @param targetNeuronBlock neuron-block, which is the target for all synapses in the section
 * @param randomSeed reference to the current seed of the randomizer
 */
template <bool doTrain>
inline void
processSynapseSection(Cluster* cluster,
                      SynapseSection* synapseSection,
                      Connection* connection,
                      Axon* transferAxon,
                      Neuron* targetNeuronBlock,
                      uint32_t& randomSeed)
{
    uint8_t pos = 0;
    Synapse* synapse = nullptr;
    Neuron* targetNeuron = nullptr;
    float halfPotential = 0.0f;
    bool condition = false;
    constexpr float createBorder = 0.05f;
    const float range = connection->potentialRange;
    float potential = transferAxon->potential - connection->lowerBound;
    float ratio = 1.0f;
    potential = range * static_cast<float>(potential > range)
                + potential * static_cast<float>(potential <= range);

    // iterate over all synapses in the section
    while (pos < SYNAPSES_PER_SECTION && potential > POTENTIAL_BORDER) {
        synapse = &synapseSection->synapses[pos];

        // create new synapse if necesarry and training is active
        if constexpr (doTrain) {
            if (synapse->targetNeuronId == UNINIT_STATE_8) {
                // because of the initialize of the section, the first position should
                // always be filled
                assert(pos > 0);
                createNewSynapse(
                    synapse, synapseSection->synapses[pos - 1].border * 2.0f, randomSeed);
            }

            if (potential < synapse->border) {
                condition = potential < (1.0f - createBorder) * synapse->border
                            && potential > createBorder * synapse->border
                            && potential < synapse->border - createBorder
                            && potential > createBorder;

                synapse->border = synapse->border * static_cast<float>(condition == false)
                                  + (synapse->border / 2.0f) * static_cast<float>(condition);
            }
        }

        ratio = 1.0f;
        if (potential < synapse->border) {
            ratio = ((1.0f / synapse->border) * potential);
        }

        targetNeuron = &targetNeuronBlock[synapse->targetNeuronId % NEURONS_PER_BLOCK];
        targetNeuron->input
            += synapse->weight1 * ratio * static_cast<float>(potential > synapse->border);

        targetNeuron = &targetNeuronBlock[(synapse->targetNeuronId + 1) % NEURONS_PER_BLOCK];
        targetNeuron->input
            += synapse->weight2 * ratio * static_cast<float>(potential > synapse->border);

        // update loop-counter
        halfPotential += static_cast<float>(pos < SYNAPSES_PER_SECTION / 2) * synapse->border;
        potential -= synapse->border;
        ++pos;
    }

    if constexpr (doTrain) {
        if (connection->splitValue == 0.0f) {
            connection->splitValue
                = halfPotential * static_cast<float>(potential > POTENTIAL_BORDER);
        }
    }
}

/**
 * @brief process all synapes of a hexagon
 *
 * @param cluster cluster, where the hexagon belongs to
 * @param hexagon pointer to current hexagon
 * @param blockId id of the current block within the hexagon
 */
template <bool doTrain>
inline void
processBlock(Cluster* cluster, Hexagon* hexagon, const uint32_t blockId)
{
    Block* blocks = getItemData<Block>(hexagon->attachedHost->blocks);
    AxonBlock* tansferAxonBlocks = &hexagon->transferAxonBlocks[0];

    Block* block = nullptr;
    SynapseSection* section = nullptr;
    const uint64_t link = hexagon->blockLinks[blockId];
    Neuron* neuronBlock = &blocks[link].neurons[0];
    Connection* connection = nullptr;
    Axon* transferAxon = nullptr;
    uint32_t randomeSeed = rand();

    if (blockId >= hexagon->header.numberOfBlocks) {
        return;
    }

    block = &blocks[hexagon->blockLinks[blockId]];

    for (uint32_t i = 0; i < NUMBER_OF_SECTIONS - 1; ++i) {
        if (block->connections[i].active == false && block->connections[i + 1].active == true) {
            block->connections[i] = block->connections[i + 1];
            block->sections[i] = block->sections[i + 1];
            block->connections[i + 1] = Connection();
            block->sections[i + 1] = SynapseSection();
            assert(block->connections[i].active == true);
            assert(block->connections[i + 1].active == false);
        }
        connection = &block->connections[i];
        transferAxon = &tansferAxonBlocks[connection->sourceBlockId].axons[connection->sourceId];

        if (connection->active == true && transferAxon->potential > POTENTIAL_BORDER) {
            section = &block->sections[i];

            processSynapseSection<doTrain>(
                cluster, section, connection, transferAxon, neuronBlock, randomeSeed);
        }
    }
}

/**
 * @brief process all neurons of a hexagon
 *
 * @param cluster cluster, where the hexagon belongs to
 * @param hexagon pointer to current hexagon
 * @param blockId id of the current block within the hexagon
 */
inline void
processNeurons(Cluster* cluster, Hexagon* hexagon, const uint32_t blockId)
{
    Block* blocks = getItemData<Block>(hexagon->attachedHost->blocks);
    ClusterSettings* clusterSettings = &cluster->clusterHeader.settings;
    const uint64_t link = hexagon->blockLinks[blockId];
    AxonBlock* axonBlock = &hexagon->axonBlocks[blockId];
    Neuron* neuronBlock = &blocks[link].neurons[0];
    Neuron* neuron = nullptr;
    Axon* axon = nullptr;

    for (uint8_t neuronId = 0; neuronId < NEURONS_PER_BLOCK; ++neuronId) {
        neuron = &neuronBlock[neuronId];
        axon = &axonBlock->axons[neuronId];

        axon->potential /= clusterSettings->neuronCooldown;
        axon->potential = static_cast<float>(axon->potential > POTENTIAL_BORDER) * axon->potential;
        neuron->refractoryTime = neuron->refractoryTime >> 1;

        if (neuron->refractoryTime == 0) {
            axon->potential += clusterSettings->potentialOverflow * neuron->input;
            neuron->refractoryTime = clusterSettings->refractoryTime;
        }

        axon->potential -= neuron->border;
        neuron->active = axon->potential > POTENTIAL_BORDER;
        axon->potential = static_cast<float>(neuron->active) * axon->potential;
        axon->potential = log2(axon->potential + 1.0f);

        neuron->input = 0.0f;
        axon->delta = 0.0f;
    }
}

/**
 * @brief process all neurons of a hexagon
 *
 * @param cluster cluster, where the hexagon belongs to
 * @param hexagon pointer to current hexagon
 * @param blockId id of the current block within the hexagon
 */
inline void
processExitNeurons(Cluster* cluster, Hexagon* hexagon, const uint32_t blockId)
{
    Block* blocks = getItemData<Block>(hexagon->attachedHost->blocks);
    ClusterSettings* clusterSettings = &cluster->clusterHeader.settings;
    const uint64_t link = hexagon->blockLinks[blockId];
    Neuron* neuronBlock = &blocks[link].neurons[0];
    Neuron* neuron = nullptr;
    AxonBlock* axonBlock = nullptr;
    Axon* axon = nullptr;

    axonBlock = &hexagon->axonBlocks[blockId];

    for (uint8_t neuronId = 0; neuronId < NEURONS_PER_BLOCK; ++neuronId) {
        neuron = &neuronBlock[neuronId];
        axon = &axonBlock->axons[neuronId];

        axon->potential = neuron->input;
        axon->potential = 1.0f / (1.0f + exp(-1.0f * axon->potential));

        neuron->input = 0.0f;
    }
}

#endif  // HANAMI_CORE_PROCESSING_H
