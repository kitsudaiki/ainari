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
 * @brief transferAxonBlocks
 *
 * @param cluster
 * @param sourceAxonBlock
 */
inline void
_transferAxonBlocks(Cluster& cluster, AxonBlock* sourceAxonBlock)
{
    if (sourceAxonBlock->targetBlockId == UNINIT_STATE_32
        || sourceAxonBlock->targetHexagonId == UNINIT_STATE_32)
    {
        return;
    }

    // std::cout<<"    _transferAxonBlocks"<<std::endl;

    Hexagon* targetHexagon = &cluster.hexagons[sourceAxonBlock->targetHexagonId];
    targetHexagon->transferAxonBlocks[sourceAxonBlock->targetBlockId] = *sourceAxonBlock;
}

/**
 * @brief transferAxonBlockToOutput
 * @param hexagon
 */
inline void
_transferAxonBlockToOutput(Hexagon* hexagon)
{
    // std::cout<<"    _transferAxonBlockToOutput: h"<<hexagon->header.hexagonId<<std::endl;

    hexagon->outputInterface->targetAxonBlocks.resize(hexagon->axonBlocks.size());

    for (uint64_t blockId = 0; blockId < hexagon->axonBlocks.size(); ++blockId) {
        AxonBlock* sourceAxonBlock = &hexagon->axonBlocks[blockId];
        hexagon->outputInterface->targetAxonBlocks[blockId] = *sourceAxonBlock;
    }
}

/**
 * @brief handleInputAxonBlocks
 * @param cluster
 * @param inputInterface
 */
template <bool doTrain>
inline void
processInputAxonBlocks(Cluster& cluster, InputInterface* inputInterface)
{
    // std::cout<<"processInputAxonBlocks"<<std::endl;

    const uint64_t targetId = inputInterface->targetHexagonId;

    cluster.hexagons[targetId].transferAxonBlocks.resize(inputInterface->inputAxons.size());

    // Counter for debug-output
    // uint32_t counter = 0;

    for (uint64_t blockId = 0; blockId < inputInterface->inputAxons.size(); ++blockId) {
        AxonBlock* axonBlock = &inputInterface->inputAxons[blockId];

        // Debug-output
        /*for(uint32_t i = 0; i < NEURONS_PER_NEURONBLOCK; i++) {
            if (axonBlock->axons[i].potential > 0.0f) {
                std::cout<<" X";
            } else {
                std::cout<<"  ";
            }
            counter++;
            if(counter == 28) {
                counter = 0;
                std::cout<<std::endl;
            }
        }*/

        if constexpr (doTrain) {
            if (axonBlock->targetBlockId == UNINIT_STATE_32) {
                axonBlock->targetHexagonId = targetId;
                axonBlock->targetBlockId = blockId;
            }
        }

        _transferAxonBlocks(cluster, axonBlock);
    }
}

/**
 * @brief handleNewAxonBlocks
 *
 * @param cluster
 * @param sourceHexagon
 */
template <bool doTrain>
inline void
processAxonBlocks(Cluster& cluster, Hexagon* hexagon, uint32_t& randomSeed)
{
    // std::cout<<"processAxonBlocks: h"<<hexagon->header.hexagonId<<std::endl;

    // handle output-interface
    if (hexagon->outputInterface != nullptr) {
        _transferAxonBlockToOutput(hexagon);
        return;
    }

    // handle normal connection
    for (uint32_t sourceBlockId = 0; sourceBlockId < hexagon->axonBlocks.size(); ++sourceBlockId) {
        AxonBlock* axon = &hexagon->axonBlocks[sourceBlockId];
        if constexpr (doTrain) {
            if (axon->targetBlockId == UNINIT_STATE_32) {
                // get and update target
                const uint64_t randPos = Hanami::pcg_hash(randomSeed) % NUMBER_OF_POSSIBLE_NEXT;
                const uint64_t targetId = hexagon->possibleHexagonTargetIds[randPos];
                const uint64_t currentSize = cluster.hexagons[targetId].transferAxonBlocks.size();
                cluster.hexagons[targetId].transferAxonBlocks.resize(currentSize + 1);

                // update information in the source axon-block
                axon->targetHexagonId = targetId;
                axon->targetBlockId = cluster.hexagons[targetId].transferAxonBlocks.size() - 1;
                axon->sourceBlockId = sourceBlockId;
                axon->sourceHexagonId = hexagon->header.hexagonId;
            }
        }

        _transferAxonBlocks(cluster, axon);
    }
}

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
    uint32_t signRand = 0;

    // set activation-border
    synapse->border = remainingW;

    // set initial active-counter for reduction-process
    synapse->activeCounter = 5;

    // set target neuron
    synapse->targetNeuronId
        = static_cast<uint16_t>(Hanami::pcg_hash(randomSeed) % NEURONS_PER_NEURONBLOCK);
    synapse->weight = (static_cast<float>(Hanami::pcg_hash(randomSeed)) / randMax) / 10.0f;

    // update weight with sign
    signRand = Hanami::pcg_hash(randomSeed) % 1000;
    synapse->weight *= static_cast<float>(1.0f - (1000.0f * sigNeg > signRand) * 2);
}

/**
 * @brief process a single synapse-section
 *
 * @param cluster cluster, where the synapseSection belongs to
 * @param synapseSection current synapse-section to process
 * @param connection pointer to the connection-object, which is related to the section
 * @param targetNeuronBlock neuron-block, which is the target for all synapses in the section
 * @param clusterSettings pointer to cluster-settings
 * @param randomSeed reference to the current seed of the randomizer
 */
inline void
synapseProcessingBackward_train(Cluster& cluster,
                                SynapseSection* synapseSection,
                                Connection* connection,
                                Axon* transferAxon,
                                Neuron* targetNeuronBlock,
                                uint32_t& randomSeed)
{
    float val = 0.0f;
    uint8_t pos = 0;
    Synapse* synapse = nullptr;
    Neuron* targetNeuron = nullptr;
    float halfPotential = 0.0f;
    bool condition = false;
    const bool isAbleToCreate = true;
    constexpr float createBorder = 0.05f;
    const float range = connection->potentialRange;
    float potential = transferAxon->potential - connection->lowerBound;
    potential = range * (potential > range) + potential * (potential <= range);

    // iterate over all synapses in the section
    while (pos < SYNAPSES_PER_SYNAPSESECTION && potential > 0.00001f) {
        synapse = &synapseSection->synapses[pos];

        // create new synapse if necesarry and training is active
        if (isAbleToCreate & (synapse->targetNeuronId == UNINIT_STATE_8)) {
            createNewSynapse(synapse, potential, randomSeed);
            cluster.enableCreation = true;
        }

        // update target-neuron
        val = synapse->weight;
        if (potential < synapse->border) {
            val *= ((1.0f / synapse->border) * potential);
            condition = cluster.enableCreation
                        && potential < (1.0f - createBorder) * synapse->border
                        && potential > createBorder * synapse->border
                        && potential < synapse->border - createBorder && potential > createBorder;
            synapse->border = synapse->border * static_cast<float>(condition == false)
                              + potential * static_cast<float>(condition);

            cluster.enableCreation = true;
        }
        targetNeuron = &targetNeuronBlock[synapse->targetNeuronId % NEURONS_PER_NEURONBLOCK];
        targetNeuron->input += val;

        // update loop-counter
        halfPotential
            += static_cast<float>(pos < SYNAPSES_PER_SYNAPSESECTION / 2) * synapse->border;
        potential -= synapse->border;
        ++pos;
    }

    if (connection->splitValue == 0.0f) {
        connection->splitValue
            = halfPotential * static_cast<float>(potential > 0.00001f && isAbleToCreate);
    }
}

/**
 * @brief process a single synapse-section
 *
 * @param synapseSection current synapse-section to process
 * @param connection pointer to the connection-object, which is related to the section
 * @param targetNeuronBlock neuron-block, which is the target for all synapses in the section
 */
inline void
synapseProcessingBackward_request(SynapseSection* synapseSection,
                                  Connection* connection,
                                  Axon* transferAxon,
                                  Neuron* targetNeuronBlock)
{
    float potential = transferAxon->potential - connection->lowerBound;
    float val = 0.0f;
    uint8_t pos = 0;
    Synapse* synapse = nullptr;
    Neuron* targetNeuron = nullptr;

    if (potential > connection->potentialRange) {
        potential = connection->potentialRange;
    }

    // iterate over all synapses in the section
    while (pos < SYNAPSES_PER_SYNAPSESECTION && potential > 0.00001f) {
        synapse = &synapseSection->synapses[pos];

        // update target-neuron
        val = synapse->weight;
        if (potential < synapse->border) {
            val *= ((1.0f / synapse->border) * potential);
        }

        targetNeuron = &targetNeuronBlock[synapse->targetNeuronId % NEURONS_PER_NEURONBLOCK];
        targetNeuron->input += val;

        // update loop-counter
        potential -= synapse->border;
        ++pos;
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
processSynapseBlock(Cluster& cluster, Hexagon* hexagon, const uint32_t blockId)
{
    // std::cout<<"processSynapseBlock: h"<<hexagon->header.hexagonId<<" : b"<<blockId<<std::endl;
    SynapseBlock* synapseBlocks = getItemData<SynapseBlock>(hexagon->attachedHost->synapseBlocks);
    AxonBlock* tansferAxonBlocks = &hexagon->transferAxonBlocks[0];

    SynapseBlock* synapseBlock = nullptr;
    SynapseSection* section = nullptr;
    const uint64_t link = hexagon->synapseBlockLinks[blockId];
    Neuron* neuronBlock = &synapseBlocks[link].neurons[0];
    Connection* connection = nullptr;
    Axon* transferAxon = nullptr;
    uint32_t randomeSeed = rand();

    if (blockId >= hexagon->header.numberOfBlocks) {
        return;
    }

    synapseBlock = &synapseBlocks[hexagon->synapseBlockLinks[blockId]];

    for (uint32_t i = 0; i < NUMBER_OF_SYNAPSESECTION - 1; ++i) {
        if (synapseBlock->connections[i].active == false
            && synapseBlock->connections[i + 1].active == true)
        {
            synapseBlock->connections[i] = synapseBlock->connections[i + 1];
            synapseBlock->sections[i] = synapseBlock->sections[i + 1];
            synapseBlock->connections[i + 1] = Connection();
            synapseBlock->sections[i + 1] = SynapseSection();
            assert(synapseBlock->connections[i].active == true);
            assert(synapseBlock->connections[i + 1].active == false);
        }
        connection = &synapseBlock->connections[i];
        transferAxon = &tansferAxonBlocks[connection->sourceBlockId].axons[connection->sourceId];

        if (connection->active == true && transferAxon->potential > 0.00001f) {
            section = &synapseBlock->sections[i];

            if constexpr (doTrain) {
                synapseProcessingBackward_train(
                    cluster, section, connection, transferAxon, neuronBlock, randomeSeed);
            }
            else {
                synapseProcessingBackward_request(section, connection, transferAxon, neuronBlock);
            }
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
processNeurons(Cluster& cluster, Hexagon* hexagon, const uint32_t blockId)
{
    // std::cout<<"processNeurons: h"<<hexagon->header.hexagonId<<" : b"<<blockId<<std::endl;
    SynapseBlock* synapseBlocks = getItemData<SynapseBlock>(hexagon->attachedHost->synapseBlocks);
    ClusterSettings* clusterSettings = &cluster.clusterHeader.settings;
    const uint64_t link = hexagon->synapseBlockLinks[blockId];
    Neuron* neuronBlock = &synapseBlocks[link].neurons[0];
    Neuron* neuron = nullptr;
    AxonBlock* axonBlock = nullptr;
    Axon* axon = nullptr;

    axonBlock = &hexagon->axonBlocks[blockId];

    for (uint8_t neuronId = 0; neuronId < NEURONS_PER_NEURONBLOCK; ++neuronId) {
        neuron = &neuronBlock[neuronId];
        axon = &axonBlock->axons[neuronId];

        axon->potential /= clusterSettings->neuronCooldown;
        axon->potential = static_cast<float>(axon->potential > 0.00001f) * axon->potential;
        neuron->refractoryTime = neuron->refractoryTime >> 1;

        if (neuron->refractoryTime == 0) {
            axon->potential += clusterSettings->potentialOverflow * neuron->input;
            neuron->refractoryTime = clusterSettings->refractoryTime;
        }

        axon->potential -= neuron->border;
        neuron->active = axon->potential > 0.00001f;
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
processExitNeurons(Cluster& cluster, Hexagon* hexagon, const uint32_t blockId)
{
    // std::cout<<"processNeurons: h"<<hexagon->header.hexagonId<<" : b"<<blockId<<std::endl;
    SynapseBlock* synapseBlocks = getItemData<SynapseBlock>(hexagon->attachedHost->synapseBlocks);
    ClusterSettings* clusterSettings = &cluster.clusterHeader.settings;
    const uint64_t link = hexagon->synapseBlockLinks[blockId];
    Neuron* neuronBlock = &synapseBlocks[link].neurons[0];
    Neuron* neuron = nullptr;
    AxonBlock* axonBlock = nullptr;
    Axon* axon = nullptr;

    axonBlock = &hexagon->axonBlocks[blockId];

    for (uint8_t neuronId = 0; neuronId < NEURONS_PER_NEURONBLOCK; ++neuronId) {
        neuron = &neuronBlock[neuronId];
        axon = &axonBlock->axons[neuronId];

        axon->potential = neuron->input;
        axon->potential = 1.0f / (1.0f + exp(-1.0f * axon->potential));

        neuron->input = 0.0f;
    }
}

#endif  // HANAMI_CORE_PROCESSING_H
