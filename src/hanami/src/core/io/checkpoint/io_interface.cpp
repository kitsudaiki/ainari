/**
 * @file        io_interface.cpp
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

#include "io_interface.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_init.h>
#include <core/processing/logical_host.h>
#include <core/processing/physical_host.h>
#include <hanami_root.h>

/**
 * @brief constructor
 */
IO_Interface::IO_Interface() {}

/**
 * @brief destructor
 */
IO_Interface::~IO_Interface() {}

/**
 * @brief serialize a cluster into an unterlying target
 *
 * @param cluster cluster to serialize
 * @param error reference for error-output
 *
 * @return OK-status, if successful, else ERROR-status
 */
ReturnStatus
IO_Interface::serialize(const Cluster& cluster, Hanami::ErrorContainer& error)
{
    const uint64_t totalClusterSize = getClusterSize(cluster);
    initLocalBuffer(totalClusterSize);
    if (initializeTarget(totalClusterSize, error) == false) {
        error.addMessage("Failed to initialize target to serialize cluster");
        return ERROR;
    }

    // write cluster-header to buffer
    if (addObjectToLocalBuffer(&cluster.clusterHeader, error) == false) {
        return ERROR;
    }

    // write number of hexagons to buffer
    const uint64_t numberOfHexagons = cluster.hexagons.size();
    if (addObjectToLocalBuffer(&numberOfHexagons, error) == false) {
        return ERROR;
    }

    // write hexagons to buffer
    for (const Hexagon& hexagon : cluster.hexagons) {
        const ReturnStatus ret = serializeHexagon(hexagon, error);
        if (ret != OK) {
            return ret;
        }
    }

    // write remaining data in the cache
    if (writeFromLocalBuffer(m_localBuffer, error) == false) {
        return ERROR;
    }

    return OK;
}

/**
 * @brief deserialize cluster from the unterlying target
 *
 * @param cluster target cluster-object
 * @param totalSize total number of bytes to initialize the local buffer
 * @param host initial host to attach the hexagons. if nullptr, use the first cpu-host
 * @param error reference for error-output
 *
 * @return return-status based on the result of the process
 */
ReturnStatus
IO_Interface::deserialize(Cluster& cluster,
                          const uint64_t totalSize,
                          LogicalHost* host,
                          Hanami::ErrorContainer& error)
{
    uint64_t positionPtr = 0;
    uint64_t numberOfHexagons = 0;
    ReturnStatus ret = OK;

    initLocalBuffer(totalSize);

    // clear old data from the cluster
    cluster.hexagons.clear();
    cluster.inputInterfaces.clear();
    cluster.outputInterfaces.clear();

    // read cluster-information
    ret = getObjectFromLocalBuffer(positionPtr, &cluster.clusterHeader, error);
    if (ret != OK) {
        return ret;
    }
    ret = getObjectFromLocalBuffer(positionPtr, &numberOfHexagons, error);
    if (ret != OK) {
        return ret;
    }

    // read hexagons
    cluster.hexagons.resize(numberOfHexagons);
    for (uint64_t i = 0; i < numberOfHexagons; i++) {
        cluster.hexagons[i].cluster = &cluster;
        if (host != nullptr) {
            cluster.hexagons[i].attachedHost = host;
        }
        else {
            cluster.hexagons[i].attachedHost = HanamiRoot::physicalHost->getFirstHost();
        }
        const ReturnStatus ret = deserializeHexagon(cluster.hexagons[i], positionPtr, error);
        if (ret != OK) {
            return ret;
        }
    }

    // initialize axon-blocks
    for (uint64_t i = 0; i < numberOfHexagons; i++) {
        Hexagon* hexagon = &cluster.hexagons[i];
        for (const AxonBlock& axonBlock : hexagon->axonBlocks) {
            if (axonBlock.targetHexagonId == UNINIT_STATE_32) {
                continue;
            }
            Hexagon* targetHexagon = &cluster.hexagons[axonBlock.targetHexagonId];
            const uint64_t currentSize = targetHexagon->transferAxonBlocks.size();
            targetHexagon->transferAxonBlocks.resize(currentSize + 1);
        }
    }

    // re-initialize neighbor-list and target-list
    connectAllHexagons(&cluster);
    initializeTargetHexagonList(&cluster);

    return OK;
}

/**
 * @brief initialize the local buffer
 *
 * @param totalSize number of bytes for the complete target for checks of the upper bound
 */
void
IO_Interface::initLocalBuffer(const uint64_t totalSize)
{
    memset(m_localBuffer.cache, 0, LOCAL_BUFFER_SIZE);
    m_localBuffer.totalSize = totalSize;
    m_localBuffer.startPos = 0;
    m_localBuffer.size = 0;
}

/**
 * @brief calculate the number of bytes necessary to serialize a specific cluster
 *
 * @param cluster cluster, of which the necessary bytes should be calculated
 *
 * @return number of bytes for the cluster
 */
uint64_t
IO_Interface::getClusterSize(const Cluster& cluster) const
{
    uint64_t size = 0;

    size += sizeof(ClusterHeader);
    size += sizeof(uint64_t);

    for (const Hexagon& hexagon : cluster.hexagons) {
        size += getHexagonSize(hexagon);
    }

    return size;
}

/**
 * @brief calculate the number of bytes necessary to serialize a specific hexagon
 *
 * @param hexagon hexagon, of which the necessary bytes should be calculated
 *
 * @return number of bytes for the hexagon
 */
uint64_t
IO_Interface::getHexagonSize(const Hexagon& hexagon) const
{
    uint64_t size = 0;

    size += sizeof(HexagonEntry);
    size += hexagon.axonBlocks.size() * sizeof(AxonBlock);
    size += hexagon.blockLinks.size() * sizeof(Block);

    if (hexagon.inputInterface != nullptr) {
        size += sizeof(InputEntry);
        size += hexagon.inputInterface->inputAxons.size() * sizeof(AxonBlock);
    }

    if (hexagon.outputInterface != nullptr) {
        size += sizeof(OutputEntry);
        size += hexagon.outputInterface->weights.size() * sizeof(OutputWeightBlock);
        size += hexagon.outputInterface->outputNeurons.size() * sizeof(OutputNeuron);
    }

    return size;
}

/**
 * @brief serialize a single hexagon
 *
 * @param hexagon hexagon, which should be serialized
 * @param error reference for error-output
 *
 * @return OK-status, if successful, else ERROR-status
 */
ReturnStatus
IO_Interface::serializeHexagon(const Hexagon& hexagon, Hanami::ErrorContainer& error)
{
    // hexagon-entry
    HexagonEntry hexagonEntry = createHexagonEntry(hexagon);
    if (addObjectToLocalBuffer(&hexagonEntry, error) == false) {
        return ERROR;
    }

    // neuron-blocks
    for (const AxonBlock& axonBlock : hexagon.axonBlocks) {
        if (addObjectToLocalBuffer(&axonBlock, error) == false) {
            return ERROR;
        }
    }

    // connection-blocks and synapse-blocks
    Block* blocks = Hanami::getItemData<Block>(hexagon.attachedHost->blocks);

    for (uint64_t pos = 0; pos < hexagon.blockLinks.size(); pos++) {
        const uint64_t synapseSectionPos = hexagon.blockLinks[pos];
        if (synapseSectionPos == UNINIT_STATE_64) {
            error.addMessage("Synapse-block-position invalid");
            return ERROR;
        }
        Block* block = &blocks[synapseSectionPos];
        if (addObjectToLocalBuffer(block, error) == false) {
            return ERROR;
        }
    }

    // input
    if (hexagon.inputInterface != nullptr) {
        // create input-entry and write it to the buffer
        InputEntry inputEntry;
        if (inputEntry.name.setName(hexagon.inputInterface->name) == false) {
            return INVALID_INPUT;
        }
        inputEntry.numberOfInputs = hexagon.inputInterface->inputAxons.size();
        inputEntry.targetHexagonId = hexagon.header.hexagonId;
        if (addObjectToLocalBuffer(&inputEntry, error) == false) {
            return ERROR;
        }

        // write input-neurons to buffer
        for (const AxonBlock& inputAxon : hexagon.inputInterface->inputAxons) {
            if (addObjectToLocalBuffer(&inputAxon, error) == false) {
                return ERROR;
            }
        }
    }

    // output
    if (hexagon.outputInterface != nullptr) {
        // create output-entry and write it to the buffer
        OutputEntry outputEntry;
        if (outputEntry.name.setName(hexagon.outputInterface->name) == false) {
            return INVALID_INPUT;
        }
        outputEntry.type = hexagon.outputInterface->type;
        outputEntry.numberOfOutputs = hexagon.outputInterface->ioBuffer.size();
        outputEntry.targetHexagonId = hexagon.header.hexagonId;
        outputEntry.numberOfWeightBlocks = hexagon.outputInterface->weights.size();
        if (addObjectToLocalBuffer(&outputEntry, error) == false) {
            return ERROR;
        }

        // write weight-blocks to buffer
        for (const OutputWeightBlock& weightBlock : hexagon.outputInterface->weights) {
            if (addObjectToLocalBuffer(&weightBlock, error) == false) {
                return ERROR;
            }
        }

        // write output-neurons to buffer
        for (const OutputNeuron& outputNeuron : hexagon.outputInterface->outputNeurons) {
            if (addObjectToLocalBuffer(&outputNeuron, error) == false) {
                return ERROR;
            }
        }
    }

    return OK;
}
/**
 * @brief IO_Interface::deserialize
 *
 * @param hexagon target-hexagon for the deserialied data
 * @param positionPtr referece to track current byte-position
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
ReturnStatus
IO_Interface::deserializeHexagon(Hexagon& hexagon,
                                 uint64_t& positionPtr,
                                 Hanami::ErrorContainer& error)
{
    const uint64_t positionOffset = positionPtr;
    ReturnStatus ret = OK;

    // hexagon-entry
    HexagonEntry hexagonEntry;
    ret = getObjectFromLocalBuffer(positionPtr, &hexagonEntry, error);
    if (ret != OK) {
        return ret;
    }
    if (checkHexagonEntry(hexagonEntry) == false) {
        error.addMessage("Input-data invalid: Hexagon-check failed.");
        return INVALID_INPUT;
    }

    hexagon.header = hexagonEntry.header;

    if (hexagonEntry.axonBlocksPos != 0) {
        // check current position
        if (positionPtr - positionOffset != hexagonEntry.axonBlocksPos) {
            error.addMessage("Input-data invalid");
            return INVALID_INPUT;
        }

        // neuron-blocks
        hexagon.axonBlocks.clear();
        const uint64_t numberOfaxonBlocks = hexagonEntry.numberOfAxonBytes / sizeof(AxonBlock);
        hexagon.axonBlocks.resize(numberOfaxonBlocks);
        for (uint64_t i = 0; i < numberOfaxonBlocks; i++) {
            ret = getObjectFromLocalBuffer(positionPtr, &hexagon.axonBlocks[i], error);
            if (ret != OK) {
                return ret;
            }
        }
    }

    if (hexagonEntry.blocksPos != 0) {
        // check current position
        if (positionPtr - positionOffset != hexagonEntry.blocksPos) {
            error.addMessage("Input-data invalid");
            return INVALID_INPUT;
        }

        // connection-blocks and synapse-blocks
        clearBlocks(hexagon);
        const uint64_t numberOfBlocks = hexagonEntry.numberOfSynapseBytes / (sizeof(Block));
        hexagon.blockLinks.resize(numberOfBlocks);
        for (uint64_t i = 0; i < numberOfBlocks; i++) {
            Block block;
            ret = getObjectFromLocalBuffer(positionPtr, &block, error);
            if (ret != OK) {
                return ret;
            }
            const uint64_t newTargetPosition = hexagon.attachedHost->blocks.addNewItem(block);
            if (newTargetPosition == UNINIT_STATE_64) {
                return ERROR;
            }
            hexagon.blockLinks[i] = newTargetPosition;
        }
    }

    // input
    if (hexagonEntry.inputInterfacesPos != 0) {
        // check current position
        if (positionPtr - positionOffset != hexagonEntry.inputInterfacesPos) {
            error.addMessage("Input-data invalid");
            return INVALID_INPUT;
        }

        InputEntry inputEntry;
        ret = getObjectFromLocalBuffer(positionPtr, &inputEntry, error);
        if (ret != OK) {
            return ret;
        }

        InputInterface inputIf;
        inputIf.name = inputEntry.name.getName();
        inputIf.targetHexagonId = hexagon.header.hexagonId;

        inputIf.inputAxons.resize(inputEntry.numberOfInputs);
        inputIf.ioBuffer.resize(inputEntry.numberOfInputs);
        for (AxonBlock& inputAxon : inputIf.inputAxons) {
            ret = getObjectFromLocalBuffer(positionPtr, &inputAxon, error);
            if (ret != OK) {
                return ret;
            }
        }

        auto ret = hexagon.cluster->inputInterfaces.try_emplace(inputIf.name, inputIf);
        if (ret.second == false) {
            error.addMessage("Input-data invalid");
            return INVALID_INPUT;
        }

        hexagon.inputInterface = &hexagon.cluster->inputInterfaces[inputIf.name];
    }

    // output
    if (hexagonEntry.outputsInterfacesPos != 0) {
        // check current position
        if (positionPtr - positionOffset != hexagonEntry.outputsInterfacesPos) {
            error.addMessage("Input-data invalid");
            return INVALID_INPUT;
        }

        OutputEntry outputEntry;
        ret = getObjectFromLocalBuffer(positionPtr, &outputEntry, error);
        if (ret != OK) {
            return ret;
        }

        OutputInterface outputIf;
        outputIf.name = outputEntry.name.getName();
        outputIf.type = outputEntry.type;
        outputIf.targetHexagonId = hexagon.header.hexagonId;
        outputIf.initBuffer(outputEntry.numberOfOutputs, 1);
        outputIf.weights.resize(outputEntry.numberOfWeightBlocks);

        // read weight-blocks
        for (OutputWeightBlock& weightBlock : outputIf.weights) {
            ret = getObjectFromLocalBuffer(positionPtr, &weightBlock, error);
            if (ret != OK) {
                return ret;
            }
        }

        // read output-neurons
        for (OutputNeuron& outputNeuron : outputIf.outputNeurons) {
            ret = getObjectFromLocalBuffer(positionPtr, &outputNeuron, error);
            if (ret != OK) {
                return ret;
            }
        }

        auto ret = hexagon.cluster->outputInterfaces.try_emplace(outputIf.name, outputIf);
        if (ret.second == false) {
            error.addMessage("Input-data invalid");
            return INVALID_INPUT;
        }

        hexagon.outputInterface = &hexagon.cluster->outputInterfaces[outputIf.name];
    }

    // check current position
    if (positionPtr - positionOffset != hexagonEntry.hexagonSize) {
        error.addMessage("Input-data invalid");
        return INVALID_INPUT;
    }

    return OK;
}

/**
 * @brief check byte-ranges within the read hexagon-entry to prevent broken input from
 *        crashing the program
 *
 * @param hexagonEntry hexagon-entry to check
 *
 * @return true, if all is valid, else false
 */
bool
IO_Interface::checkHexagonEntry(const HexagonEntry& hexagonEntry)
{
    // check order
    if (hexagonEntry.axonBlocksPos != 0 && hexagonEntry.axonBlocksPos < sizeof(HexagonEntry)) {
        return false;
    }
    if (hexagonEntry.blocksPos != 0 && hexagonEntry.blocksPos < hexagonEntry.axonBlocksPos) {
        return false;
    }
    if (hexagonEntry.blocksPos == 0 && hexagonEntry.inputInterfacesPos < hexagonEntry.blocksPos) {
        return false;
    }
    if (hexagonEntry.blocksPos == 0 && hexagonEntry.outputsInterfacesPos < hexagonEntry.blocksPos) {
        return false;
    }

    // check against total hexagon size
    if (hexagonEntry.axonBlocksPos >= hexagonEntry.hexagonSize) {
        return false;
    }
    if (hexagonEntry.blocksPos >= hexagonEntry.hexagonSize) {
        return false;
    }
    if (hexagonEntry.inputInterfacesPos >= hexagonEntry.hexagonSize) {
        return false;
    }
    if (hexagonEntry.outputsInterfacesPos >= hexagonEntry.hexagonSize) {
        return false;
    }

    // check positions
    if (hexagonEntry.inputInterfacesPos == 0
        && hexagonEntry.axonBlocksPos + hexagonEntry.numberOfAxonBytes != hexagonEntry.blocksPos)
    {
        return false;
    }
    if (hexagonEntry.blocksPos + hexagonEntry.numberOfSynapseBytes
            != hexagonEntry.inputInterfacesPos
        && hexagonEntry.blocksPos + hexagonEntry.numberOfSynapseBytes
               != hexagonEntry.outputsInterfacesPos
        && hexagonEntry.blocksPos + hexagonEntry.numberOfSynapseBytes != hexagonEntry.hexagonSize)
    {
        return false;
    }

    // check sizes compared to the object-types
    if (hexagonEntry.numberOfAxonBytes % sizeof(AxonBlock) != 0) {
        return false;
    }
    if (hexagonEntry.numberOfSynapseBytes % (sizeof(Block)) != 0) {
        return false;
    }

    if (hexagonEntry.inputInterfacesPos > 0
        && (hexagonEntry.hexagonSize - hexagonEntry.inputInterfacesPos - sizeof(InputEntry))
                   % sizeof(AxonBlock)
               != 0)
    {
        return false;
    }
    if (hexagonEntry.outputsInterfacesPos > 0
        && (hexagonEntry.hexagonSize - hexagonEntry.outputsInterfacesPos - sizeof(OutputEntry))
                   % sizeof(OutputNeuron)
               != 0)
    {
        return false;
    }

    // check size against dimentsions in hexagon-header
    const uint64_t numberOfConnectionBlocks = hexagonEntry.numberOfSynapseBytes / (sizeof(Block));
    if (numberOfConnectionBlocks != hexagonEntry.header.numberOfBlocks) {
        return false;
    }

    return true;
}

/**
 * @brief create a new hexagon-entry for a hexagon
 *
 * @param hexagon hexagon for which a new entry should be created
 *
 * @return new created hexagon-entry
 */
IO_Interface::HexagonEntry
IO_Interface::createHexagonEntry(const Hexagon& hexagon)
{
    HexagonEntry hexagonEntry;

    const uint64_t hexagonSize = getHexagonSize(hexagon);
    uint64_t posCounter = 0;

    hexagonEntry.header = hexagon.header;
    hexagonEntry.hexagonSize = hexagonSize;
    posCounter += sizeof(HexagonEntry);

    if (hexagon.axonBlocks.size() > 0) {
        hexagonEntry.axonBlocksPos = posCounter;
        hexagonEntry.numberOfAxonBytes = hexagon.axonBlocks.size() * sizeof(AxonBlock);
        posCounter += hexagonEntry.numberOfAxonBytes;
    }

    if (hexagon.blockLinks.size() > 0) {
        hexagonEntry.blocksPos = posCounter;
        hexagonEntry.numberOfSynapseBytes = hexagon.blockLinks.size() * sizeof(Block);
        posCounter += hexagonEntry.numberOfSynapseBytes;
    }

    if (hexagon.inputInterface != nullptr) {
        hexagonEntry.inputInterfacesPos = posCounter;
        hexagonEntry.numberOfInputsBytes
            = sizeof(InputEntry) + (hexagon.inputInterface->inputAxons.size() * sizeof(AxonBlock));
    }

    if (hexagon.outputInterface != nullptr) {
        hexagonEntry.outputsInterfacesPos = posCounter;
        hexagonEntry.numberOfOutputBytes
            = sizeof(OutputEntry)
              + (hexagon.outputInterface->weights.size() * sizeof(OutputWeightBlock))
              + (hexagon.outputInterface->outputNeurons.size() * sizeof(OutputNeuron));
    }

    return hexagonEntry;
}

/**
 * @brief delete all connection-blocks and linked synapse-blocks of a hexagon to clear the content
 *        before reading new data into it
 *
 * @param hexagon reference to the hexagon to clear
 */
void
IO_Interface::clearBlocks(Hexagon& hexagon)
{
    for (const uint64_t synpaseBlockPos : hexagon.blockLinks) {
        hexagon.attachedHost->blocks.deleteItem(synpaseBlockPos);
    }
    hexagon.blockLinks.clear();
}
