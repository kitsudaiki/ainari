/**
 * @file        reduction.h
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

#ifndef HANAMI_CORE_REDUCTION_H
#define HANAMI_CORE_REDUCTION_H

#include <core/cluster/cluster.h>
#include <core/cluster/objects.h>
#include <core/processing/logical_host.h>
#include <hanami_root.h>

/**
 * @brief backpropagate a synapse-section
 *
 * @param section current synapse-section
 */
inline bool
reduceSection(SynapseSection* section)
{
    Synapse* synapse;
    uint8_t exist = 0;
    uint8_t pos = 0;

    for (pos = 0; pos < SYNAPSES_PER_SECTION; ++pos) {
        synapse = &section->synapses[pos];

        if (synapse->targetNeuronId != UNINIT_STATE_8) {
            synapse->activeCounter -= static_cast<uint8_t>(synapse->activeCounter < 10);

            // handle active-counter
            if (synapse->activeCounter == 0) {
                if (pos < SYNAPSES_PER_SECTION - 1) {
                    section->synapses[pos] = section->synapses[pos + 1];
                    section->synapses[pos + 1] = Synapse();
                }
                else {
                    section->synapses[pos] = Synapse();
                }
            }
            else {
                exist++;
            }
        }
    }

    // return true;
    return exist != 0;
}

/**
 * @brief backpropagate connections
 *
 * @param hexagon pointer to current hexagon
 * @param neuronBlocks pointer to neuron-blocks
 * @param blocks pointer to synapse-blocks
 * @param blockId id of the current block within the hexagon
 */
inline void
reduceConnections(Hexagon* hexagon, Hexagon* hexagons, Block* blocks, const uint32_t blockId)
{
    /*Connection* connection = nullptr;
    Neuron* sourceNeuron = nullptr;
    NeuronBlock* sourceNeuronBlock = nullptr;
    Hexagon* sourceHexagon = nullptr;
    ConnectionBlock* connectionBlock = nullptr;
    SynapseSection* synapseSection = nullptr;
    const uint32_t dimX = hexagon->header.numberOfBlocks;

    if (blockId >= dimX) {
        return;
    }

    connectionBlock = &hexagon->connectionBlocks[blockId];
    const uint64_t blockLink = hexagon->blockLinks[blockId];

    for (uint32_t i = 0; i < NUMBER_OF_SYNAPSESECTION; ++i) {
        connection = &connectionBlock->connections[i];
        if (connection->origin.blockId == UNINIT_STATE_16) {
            continue;
        }

        synapseSection = &blocks[blockLink].sections[i];
        sourceHexagon = &hexagons[connection->origin.hexagonId];
        sourceNeuronBlock = &sourceHexagon->neuronBlocks[connection->origin.blockId];
        sourceNeuron = &sourceNeuronBlock->neurons[connection->origin.neuronId];

        // if section is complete empty, then erase it
        if (reduceSection(synapseSection) == false) {
            // mark current connection as available again
            connection->origin.blockId = UNINIT_STATE_16;
            connection->origin.neuronId = UNINIT_STATE_8;
        }
    }*/
}

/**
 * @brief reduce all synapses within the cluster and delete them, if the reach a deletion-border
 */
inline void
reduceCluster(Cluster& cluster, const uint32_t hexagonId, const uint32_t blockId)
{
    Hexagon* hexagon = &cluster.hexagons[hexagonId];
    if (hexagon->header.isInputHexagon) {
        return;
    }

    Block* blocks = Hanami::getItemData<Block>(hexagon->attachedHost->blocks);
    const uint32_t numberOfHexagons = cluster.hexagons.size();
    reduceConnections(hexagon, &cluster.hexagons[0], blocks, blockId);
}

#endif  // HANAMI_CORE_REDUCTION_H
