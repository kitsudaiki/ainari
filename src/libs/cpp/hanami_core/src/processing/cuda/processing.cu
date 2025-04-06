/**
 * @file        gpu_kernel.cu
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

#include <iostream>
#include <chrono>
#include <math.h>
#include <climits>
#include <float.h>

#include <cuda_runtime_api.h>

#include "../../cluster/objects.h"

/**
 * @brief function for generating random-values
 *        coming from this website:
 *            https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering/
 *
 * @param input seed for random value
 *
 * @return random value
 */
__device__ __forceinline__
uint32_t pcg_hash(const uint32_t input)
{
    const uint32_t state = input * 747796405u + 2891336453u;
    const uint32_t word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

//==================================================================================================
//==================================================================================================
//==================================================================================================

/**
 * @brief initialize a new synpase
 *
 * @param block source-neuron-block, which is only used to hold the randamo-value
 * @param synapse pointer to the synapse, which should be (re-) initialized
 * @param clusterSettings pointer to the cluster-settings
 * @param remainingW new weight for the synapse
 * @param randomValues pointer to the buffer with all randow-values
 */
__device__ __forceinline__ void
createNewSynapse(Synapse* synapse,
                 const ClusterSettings* clusterSettings,
                 const float remainingW,
                 uint32_t& randomSeed)
{
    const float randMax = static_cast<float>(RAND_MAX);
    uint32_t signRand = 0;
    const float sigNeg = 0.5f;

    // set activation-border
    synapse->border = remainingW;

    // set initial active-counter for reduction-process
    synapse->activeCounter = 5;

    // set target neuron
    randomSeed = pcg_hash(randomSeed);
    synapse->targetNeuronId = static_cast<uint16_t>(randomSeed % NEURONS_PER_NEURONBLOCK);

    randomSeed = pcg_hash(randomSeed);
    synapse->weight = (static_cast<float>(randomSeed) / randMax) / 10.0f;

    // update weight with sign
    randomSeed = pcg_hash(randomSeed);
    signRand = randomSeed % 1000;
    synapse->weight *= static_cast<float>(1.0f - (1000.0f * sigNeg > signRand) * 2);
}

/**
 * @brief process a single synapse-section
 *
 * @param synapseSection current synapse-section to process
 * @param connection pointer to the connection-object, which is related to the section
 * @param targetNeuronBlock neuron-block, which is the target for all synapses in the section
 * @param sourceNeuron pointer to source-neuron, which had triggered the section
 * @param originLocation location of the source-neuron to mark updates
 * @param clusterSettings pointer to cluster-settings
 * @param randomValues pointer to the list with all random-values
 * @param localMem pointer to shared-memory, which should be used by the processing thread
 */
template <bool doTrain>
__device__ __forceinline__ void
synapseProcessingBackward(SynapseSection* synapseSection,
                          Connection* connection,
                          NeuronBlock* targetNeuronBlock,
                          ClusterSettings* clusterSettings,
                          uint32_t& randomSeed)
{
    __shared__ float localPotential[64];
    localPotential[threadIdx.x] = connection->potential - connection->lowerBound;

    float val = 0.0f;
    uint8_t pos = 0;
    Synapse* synapse = nullptr;
    float halfPotential = 0.0f;
    const bool isAbleToCreate = connection->origin.isInput || clusterSettings->enableCreation;

    //for(uint32_t i = 0; i < SYNAPSES_PER_SYNAPSESECTION; ++i) {
    //    synapseSection->synapses[i].tempValue = 0.0f;
    //}

    // iterate over all synapses in the section
    while (pos < SYNAPSES_PER_SYNAPSESECTION && localPotential[threadIdx.x] > 0.01f) {
        synapse = &synapseSection->synapses[pos];

        if constexpr (doTrain) {
            // create new synapse if necesarry and training is active
            if (synapse->targetNeuronId == UNINIT_STATE_8) {
                createNewSynapse(synapse,
                                 clusterSettings,
                                 localPotential[threadIdx.x],
                                 randomSeed);
                clusterSettings->enableCreation = true;
            }

            // split synapse, if necessary
            if (isAbleToCreate && localPotential[threadIdx.x] < (0.5f + connection->tollerance) * synapse->border
                && localPotential[threadIdx.x] > (0.5f - connection->tollerance) * synapse->border)
            {
                synapse->border /= 1.5f;
                synapse->weight /= 1.5f;
                connection->tollerance /= 1.2f;
                clusterSettings->enableCreation = true;
            }
        }

        if (synapse->targetNeuronId != UNINIT_STATE_8) {
            // update target-neuron
            val = synapse->weight;
            if (localPotential[threadIdx.x] < synapse->border) {
                val *= ((1.0f / synapse->border) * localPotential[threadIdx.x]);
            }
            synapseSection->synapses[synapse->targetNeuronId].tempValue += val;
        }

        // update loop-counter
        halfPotential
            += static_cast<float>(pos < SYNAPSES_PER_SYNAPSESECTION / 2) * synapse->border;
        localPotential[threadIdx.x] -= synapse->border;
        ++pos;
    }

    if constexpr (doTrain) {
        if (localPotential[threadIdx.x] > 0.00001f && isAbleToCreate) {
            connection->splitValue = halfPotential;
        }
    }
}

/**
 * @brief processSynapses
 *
 * @param neuronBlocks pointer to neuron-blocks in gpu-memory
 * @param synapseBlocks pointer to synapse-blocks in gpu-memory
 * @param connectionBlocks pointer to connection-blocks in gpu-memory
 * @param clusterSettings pointer to cluster-settingss in gpu-memory
 * @param randomValues pointer to list with random-values in gpu-memory
 * @param neuronBlockPos position-offset within the neuron-block-buffer
 * @param dimY number of connections-blocks in y-direction
 */
template <bool doTrain>
__global__ void
processSynapses(NeuronBlock* neuronBlocks,
                ConnectionBlock* connectionBlocks,
                uint64_t* synapseBlockLinks,
                SynapseBlock* synapseBlocks,
                ClusterSettings* clusterSettings,
                uint32_t randomeSeed)
{
    randomeSeed += (blockIdx.x * blockDim.x) + threadIdx.x;

    // init global pointers
    NeuronBlock* targetNeuronBlock = &neuronBlocks[blockIdx.x];
    ConnectionBlock* connectionBlock = &connectionBlocks[blockIdx.x];
    SynapseBlock* synapseBlock = &synapseBlocks[synapseBlockLinks[blockIdx.x]];
    Connection* connection = &connectionBlock->connections[threadIdx.x];

    if (connection->origin.blockId != UNINIT_STATE_16 && connection->potential > 0.0f) {
        synapseProcessingBackward<doTrain>(&synapseBlock->sections[threadIdx.x],
                                           connection,
                                           targetNeuronBlock,
                                           clusterSettings,
                                           randomeSeed);
    }
}

/**
 * @brief process neurons
 *
 * @param neuronBlocks pointer to neuron-blocks in gpu-memory
 * @param synapseBlocks pointer to synapse-blocks in gpu-memory
 * @param connectionBlocks pointer to connection-blocks in gpu-memory
 * @param clusterSettings pointer to cluster-settings in gpu-memory
 * @param neuronBlockPos position-offset within the neuron-block-buffer
 * @param dimY number of connections-blocks in y-direction
 * @param isOutputHexagon true, if current hexagon is an output-hexagon
 */
template <bool doTrain>
__global__ void
processNeurons(NeuronBlock* neuronBlocks,
               uint64_t* synapseBlockLinks,
               SynapseBlock* synapseBlocks,
               ClusterSettings* clusterSettings,
               const bool isOutputHexagon)
{
    // init shared memory
    __shared__ float localInputs[64];
    localInputs[threadIdx.x] = 0.0f;

    // init global pointers
    NeuronBlock* targetNeuronBlock = &neuronBlocks[blockIdx.x];
    SynapseBlock* synapseBlock = &synapseBlocks[synapseBlockLinks[blockIdx.x]];

    // copy input-values of all releaded synpase-blocks into the neurons
    for (uint32_t i = 0; i < NUMBER_OF_SYNAPSESECTION; ++i) {
        localInputs[threadIdx.x] += synapseBlock->sections[i].synapses[threadIdx.x].tempValue;
        synapseBlock->sections[i].synapses[threadIdx.x].tempValue = 0.0f;
    }

    Neuron* neuron = &targetNeuronBlock->neurons[threadIdx.x];
    neuron->input = localInputs[threadIdx.x];

    // process neuron-content
    if(isOutputHexagon == false)
    {
        neuron->potential /= clusterSettings->neuronCooldown;
        neuron->refractoryTime = neuron->refractoryTime >> 1;

        if (neuron->refractoryTime == 0) {
            neuron->potential = clusterSettings->potentialOverflow * neuron->input;
            neuron->refractoryTime = clusterSettings->refractoryTime;
        }

        neuron->potential -= neuron->border;
        neuron->active = neuron->potential > 0.0f;
        neuron->potential = static_cast<float>(neuron->active) * neuron->potential;
        neuron->input = 0.0f;
        neuron->potential = log2(neuron->potential + 1.0f);
    }
}

/**
 * @brief process all normal- and output-hexagons and train them, if wanted.
 *
 * @param gpuPointer handle with all gpu-pointer of the cluster
 * @param hexagons pointer to local hexagons
 * @param numberOfHexagons number of hexagons
 * @param neuronBlocks pointer to local neuron-block
 * @param numberOfNeuronBlocks number of neuron-blokcs
 * @param doTrain true to run a taining-process
 */
extern "C"
void
processing_CUDA(Hexagon* hexagon,
                SynapseBlock* synapseBlocks,
                const bool doTrain)
{
    cudaSetDevice(hexagon->cudaPointer.deviceId);
    uint32_t randomeSeed = rand();

    if (hexagon->header.isInputHexagon) {
        return;
    }

    // copy necessary data from host to gpu
    cudaMemcpy(hexagon->cudaPointer.connectionBlocks,
               &hexagon->connectionBlocks[0],
               hexagon->connectionBlocks.size() * sizeof(ConnectionBlock),
               cudaMemcpyHostToDevice);

    if (doTrain)
    {
        processSynapses<true><<<hexagon->header.numberOfBlocks, NUMBER_OF_SYNAPSESECTION>>>(
            hexagon->cudaPointer.neuronBlocks,
            hexagon->cudaPointer.connectionBlocks,
            hexagon->cudaPointer.synapseBlockLinks,
            synapseBlocks,
            hexagon->cudaPointer.clusterSettings,
            randomeSeed + hexagon->header.hexagonId);

        processNeurons<true><<<hexagon->header.numberOfBlocks, NEURONS_PER_NEURONBLOCK>>>(
            hexagon->cudaPointer.neuronBlocks,
            hexagon->cudaPointer.synapseBlockLinks,
            synapseBlocks,
            hexagon->cudaPointer.clusterSettings,
            hexagon->header.isOutputHexagon);
    }
    else
    {
        processSynapses<false><<<hexagon->header.numberOfBlocks, NUMBER_OF_SYNAPSESECTION>>>(
            hexagon->cudaPointer.neuronBlocks,
            hexagon->cudaPointer.connectionBlocks,
            hexagon->cudaPointer.synapseBlockLinks,
            synapseBlocks,
            hexagon->cudaPointer.clusterSettings,
            randomeSeed + hexagon->header.hexagonId);

        processNeurons<false><<<hexagon->header.numberOfBlocks, NEURONS_PER_NEURONBLOCK>>>(
            hexagon->cudaPointer.neuronBlocks,
            hexagon->cudaPointer.synapseBlockLinks,
            synapseBlocks,
            hexagon->cudaPointer.clusterSettings,
            hexagon->header.isOutputHexagon);
    }

    // copy resulting data back to host
    cudaMemcpy(&hexagon->neuronBlocks[0],
               hexagon->cudaPointer.neuronBlocks,
               hexagon->neuronBlocks.size() * sizeof(NeuronBlock),
               cudaMemcpyDeviceToHost);
}
