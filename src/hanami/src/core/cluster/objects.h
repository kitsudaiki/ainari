/**
 * @file        objects.h
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

#ifndef HANAMI_CORE_SEGMENT_OBJECTS_H
#define HANAMI_CORE_SEGMENT_OBJECTS_H

#include <assert.h>
#include <hanami_common/structs.h>
#include <hanami_common/uuid.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <cstdlib>
#include <limits>
#include <string>
#include <vector>

class Cluster;
class LogicalHost;

// const predefined values
#define UNINIT_STATE_64 0xFFFFFFFFFFFFFFFF
#define UNINIT_STATE_32 0xFFFFFFFF
#define UNINIT_STATE_16 0xFFFF
#define UNINIT_STATE_8 0xFF

#define UNINTI_POINT_32 0x0FFFFFFF

// network-predefines
#define SYNAPSES_PER_SYNAPSESECTION 128
#define NUMBER_OF_SYNAPSESECTION 512
#define NEURONS_PER_NEURONBLOCK 128
#define POSSIBLE_NEXT_AXON_STEP 80
#define NUMBER_OF_POSSIBLE_NEXT 86
#define NUMBER_OF_OUTPUT_CONNECTIONS 7

//==================================================================================================

struct ClusterSettings {
    float backpropagationBorder = 0.001f;
    float potentialOverflow = 1.0f;

    float neuronCooldown = 1000000000.0f;
    uint32_t refractoryTime = 1;
    int32_t maxConnectionDistance = 1;
    bool enableReduction = false;
    bool enableCreation = false;

    uint8_t padding[42];

    bool operator==(ClusterSettings& rhs)
    {
        if (backpropagationBorder != rhs.backpropagationBorder) {
            return false;
        }
        if (potentialOverflow != rhs.potentialOverflow) {
            return false;
        }
        if (neuronCooldown != rhs.neuronCooldown) {
            return false;
        }
        if (refractoryTime != rhs.refractoryTime) {
            return false;
        }
        if (maxConnectionDistance != rhs.maxConnectionDistance) {
            return false;
        }
        if (enableReduction != rhs.enableReduction) {
            return false;
        }
        // enableCreation is only a temporary value and not relevant for this comparism
        return true;
    }

    bool operator!=(ClusterSettings& rhs) { return (*this == rhs) == false; }
};
static_assert(sizeof(ClusterSettings) == 64);

//==================================================================================================

struct ClusterHeader {
    uint8_t objectType = 0;
    uint8_t version = 1;
    uint8_t padding[2];

    Hanami::NameEntry name;

    uint64_t staticDataSize = 0;
    UUID uuid;

    ClusterSettings settings;

    uint8_t padding2[136];

    bool operator==(ClusterHeader& rhs)
    {
        if (objectType != rhs.objectType) {
            return false;
        }
        if (version != rhs.version) {
            return false;
        }
        if (name != rhs.name) {
            return false;
        }
        if (staticDataSize != rhs.staticDataSize) {
            return false;
        }
        if (strncmp(uuid.uuid, rhs.uuid.uuid, 37) != 0) {
            return false;
        }
        if (settings != rhs.settings) {
            return false;
        }

        return true;
    }
};
static_assert(sizeof(ClusterHeader) == 512);

//==================================================================================================
//==================================================================================================
//==================================================================================================

struct Axon {
    float potential = 0.0f;
    float delta = 0.0f;
    uint8_t activeCounter = 0;
} __attribute__((packed));
static_assert(sizeof(Axon) == 9);

//==================================================================================================

struct AxonBlock {
    Axon axons[NEURONS_PER_NEURONBLOCK];

    uint8_t padding[880];

    uint32_t targetHexagonId = UNINIT_STATE_32;
    uint32_t targetBlockId = UNINIT_STATE_32;

    uint32_t sourceHexagonId = UNINIT_STATE_32;
    uint32_t sourceBlockId = UNINIT_STATE_32;

    AxonBlock() { std::fill_n(axons, NEURONS_PER_NEURONBLOCK, Axon()); }
};
static_assert(sizeof(AxonBlock) == 2048);

//==================================================================================================

struct Connection {
    float lowerBound = 0.0f;
    float potentialRange = std::numeric_limits<float>::max();
    float splitValue = 0.0;

    uint32_t sourceBlockId = UNINIT_STATE_32;
    uint8_t sourceId = UNINIT_STATE_8;

    bool active = false;
    uint8_t padding[6];
};
static_assert(sizeof(Connection) == 24);

//==================================================================================================

struct Synapse {
    float weight = 0.0f;
    float border = 0.0f;
    float tempValue = 0.0f;
    uint8_t padding2[2];
    uint8_t activeCounter = 0;
    uint8_t targetNeuronId = UNINIT_STATE_8;
};
static_assert(sizeof(Synapse) == 16);

//==================================================================================================

struct SynapseSection {
    Synapse synapses[SYNAPSES_PER_SYNAPSESECTION];

    SynapseSection() { std::fill_n(synapses, SYNAPSES_PER_SYNAPSESECTION, Synapse()); }
};
static_assert(sizeof(SynapseSection) == 2048);

//==================================================================================================

struct SynapseBlock {
    SynapseSection sections[NUMBER_OF_SYNAPSESECTION];
    Connection connections[NUMBER_OF_SYNAPSESECTION];

    SynapseBlock()
    {
        std::fill_n(sections, NUMBER_OF_SYNAPSESECTION, SynapseSection());
        std::fill_n(connections, NUMBER_OF_SYNAPSESECTION, Connection());
    }
};
static_assert(sizeof(SynapseBlock)
              == (NUMBER_OF_SYNAPSESECTION * 2048) + (NUMBER_OF_SYNAPSESECTION * 24));

//==================================================================================================

struct Neuron {
    float input = 0.0f;
    float border = 0.0f;

    uint8_t refractoryTime = 1;
    uint8_t active = 0;
    uint8_t inUse = 0;

    uint8_t padding[21];
};
static_assert(sizeof(Neuron) == 32);

//==================================================================================================

struct NeuronBlock {
    Neuron neurons[NEURONS_PER_NEURONBLOCK];

    NeuronBlock() { std::fill_n(neurons, NEURONS_PER_NEURONBLOCK, Neuron()); }
};
static_assert(sizeof(NeuronBlock) == 4096);

//==================================================================================================

struct InputNeuron {
    uint32_t neuronId = UNINIT_STATE_32;
    float value = 0.0f;
};
static_assert(sizeof(InputNeuron) == 8);

//==================================================================================================
//==================================================================================================
//==================================================================================================

struct OutputTargetLocationPtr {
    float connectionWeight = 0.0f;
    uint32_t blockId = UNINIT_STATE_32;
    uint16_t neuronId = UNINIT_STATE_8;
    uint8_t padding[6];
};
static_assert(sizeof(OutputTargetLocationPtr) == 16);

//==================================================================================================

struct OutputNeuron {
    OutputTargetLocationPtr targets[NUMBER_OF_OUTPUT_CONNECTIONS];
    float outputVal = 0.0f;
    float exprectedVal = 0.0f;
    uint8_t padding[8];
};
// static_assert(sizeof(OutputNeuron) == 128);

//==================================================================================================

struct OutputInterface {
    std::string name = "";
    uint32_t targetHexagonId = UNINIT_STATE_32;
    std::vector<OutputNeuron> outputNeurons;
    std::vector<AxonBlock> targetAxonBlocks;
    std::vector<float> ioBuffer;
    OutputType type = PLAIN_OUTPUT;

    void initBuffer(uint64_t expectedSize, const uint64_t timeLength)
    {
        assert(timeLength >= 1);
        expectedSize += (timeLength - 1);
        if (ioBuffer.size() != expectedSize) {
            ioBuffer.resize(expectedSize);
        }
        if (type == FLOAT_OUTPUT) {
            expectedSize *= 32;
        }
        if (type == INT_OUTPUT) {
            expectedSize *= 64;
        }
        outputNeurons.resize(expectedSize);
    }
};

//==================================================================================================

struct InputInterface {
    std::string name = "";
    uint32_t targetHexagonId = UNINIT_STATE_32;
    std::vector<AxonBlock> inputAxons;
    std::vector<float> ioBuffer;

    void initBuffer(uint64_t expectedSize, const uint64_t timeLength)
    {
        assert(timeLength >= 1);
        if (ioBuffer.size() != expectedSize) {
            ioBuffer.resize(expectedSize);
        }
        expectedSize += (timeLength - 1);
        expectedSize /= NEURONS_PER_NEURONBLOCK;
        expectedSize++;
        if (inputAxons.size() < expectedSize) {
            const uint64_t oldSize = 0;
            inputAxons.resize(expectedSize);
            for (uint64_t i = oldSize; i < inputAxons.size(); i++) {
                inputAxons[i] = AxonBlock();
            }
        }
    }
};

//==================================================================================================
//==================================================================================================
//==================================================================================================

struct CudaHexagonPointer {
    uint32_t deviceId = 0;

    NeuronBlock* neuronBlocks = nullptr;
    uint64_t* synapseBlockLinks = nullptr;

    ClusterSettings* clusterSettings = nullptr;
};

//==================================================================================================

struct HexagonHeader {
    uint32_t hexagonId = UNINIT_STATE_32;
    bool isInputHexagon = false;
    bool isOutputHexagon = false;
    bool isBinaryInput = false;
    uint8_t padding[5];
    uint32_t axonTarget = UNINIT_STATE_32;
    uint32_t numberOfFreeSections = 0;
    uint32_t numberOfBlocks = 0;
    Hanami::Position hexagonPos;

    bool operator==(HexagonHeader& rhs)
    {
        if (hexagonId != rhs.hexagonId) {
            return false;
        }
        if (isInputHexagon != rhs.isInputHexagon) {
            return false;
        }
        if (isOutputHexagon != rhs.isOutputHexagon) {
            return false;
        }
        if (numberOfBlocks != rhs.numberOfBlocks) {
            return false;
        }
        if (hexagonPos != rhs.hexagonPos) {
            return false;
        }

        return true;
    }
};
static_assert(sizeof(HexagonHeader) == 40);

//==================================================================================================

struct ClusterMetrics {
    uint64_t numberOfBlocks = 0;
    uint64_t numberOfSections = 0;
};

//==================================================================================================

struct Hexagon {
    HexagonHeader header;

    Cluster* cluster = nullptr;
    InputInterface* inputInterface = nullptr;
    OutputInterface* outputInterface = nullptr;
    LogicalHost* attachedHost;

    CudaHexagonPointer cudaPointer;

    std::vector<NeuronBlock> neuronBlocks;
    std::vector<AxonBlock> axonBlocks;
    std::vector<AxonBlock> transferAxonBlocks;
    std::vector<uint64_t> synapseBlockLinks;

    bool wasResized = false;
    uint32_t possibleHexagonTargetIds[NUMBER_OF_POSSIBLE_NEXT];
    uint32_t neighbors[12];

    Hexagon() { std::fill_n(neighbors, 12, UNINIT_STATE_32); }
    ~Hexagon(){};

    Hexagon& operator=(const Hexagon&) = delete;
};

//==================================================================================================

#endif  // HANAMI_CORE_SEGMENT_OBJECTS_H
