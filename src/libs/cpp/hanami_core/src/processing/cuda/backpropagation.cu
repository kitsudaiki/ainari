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

#include <cuda_runtime_api.h>

#include "../../cluster/objects.h"

/**
 * @brief process neuron in backpropagation-steup
 *
 * @param neuronBlocks pointer to neuron-blocks in gpu-memory
 * @param tempNeuronBlocks pointer to temp-buffer of neuron-blocks in gpu-memory
 * @param neuronBlockPos position-offset within the neuron-block-buffer
 */
__global__ void
backpropagateNeurons(NeuronBlock* neuronBlocks)
{
    NeuronBlock* targetNeuronBlock = &neuronBlocks[blockIdx.x];
    Neuron* targetNeuron = &targetNeuronBlock->neurons[threadIdx.x];

    if (targetNeuron->active) {
        targetNeuron->delta *= 1.4427f * pow(0.5f, targetNeuron->potential);
    }
}

/**
 * @brief backpropagate a synapse-section
 *
 * @param section current synapse-section
 * @param connection current connection related to the synapse-section
 * @param targetTempBlock temp-value-block of the target neuron-block
 * @param sourceNeuron source-neuron, which triggered the section
 * @param sourceTempNeuron temp-balue block of the source-neuron
 */
__device__ __forceinline__ void
backpropagateNeuron(SynapseSection* section,
                    Connection* connection,
                    NeuronBlock* targetBlock)
{
    __shared__ float localDelta[NUMBER_OF_SYNAPSESECTION];
    __shared__ float localTotalDeltas[NUMBER_OF_SYNAPSESECTION];
    __shared__ float localPotential[NUMBER_OF_SYNAPSESECTION];

    // init values
    localPotential[threadIdx.x] = connection->potential - connection->lowerBound;
    Synapse* synapse = nullptr;
    Neuron* targetNeuron = nullptr;
    constexpr float trainValue = 0.05f;
    localTotalDeltas[threadIdx.x] = 0.0f;
    float valid = 0.0f;
    uint8_t active = 0;

    // iterate over all synapses in the section
    for (uint16_t pos = 0; pos < SYNAPSES_PER_SYNAPSESECTION; pos++) {
        synapse = &section->synapses[pos];

        if (synapse->targetNeuronId != UNINIT_STATE_8) {
            targetNeuron = &targetBlock->neurons[synapse->targetNeuronId];
            active = (targetNeuron->potential > 0.0f) == (synapse->weight > 0.0f);
            synapse->activeCounter += active * static_cast<uint8_t>(synapse->activeCounter < 10);

            // calculate new delta
            localDelta[threadIdx.x] = targetNeuron->delta * synapse->weight;
            /*if (localPotential[threadIdx.x] < synapse->border) {
                localDelta[threadIdx.x] *= (1.0f / synapse->border) * localPotential[threadIdx.x];
            }*/

            // update values
            valid = (float)(localPotential[threadIdx.x] > 0.01f);
            synapse->weight -= trainValue * targetNeuron->delta * valid;
            localTotalDeltas[threadIdx.x] += localDelta[threadIdx.x] * valid;
            localPotential[threadIdx.x] -= synapse->border;
        }
    }
}

/**
 * @brief backpropagate connections
 *
 * @param neuronBlocks pointer to neuron-blocks in gpu-memory
 * @param tempNeuronBlocks pointer to temp-values of the neuron-blocks in gpu-memory
 * @param synapseBlocks pointer to synapse-blocks in gpu-memory
 * @param connectionBlocks pointer to connection-blocks in gpu-memory
 * @param neuronBlockPos position-offset within the neuron-block-buffer
 * @param dimY number of connections-blocks in y-direction
 */
__global__ void
backpropagateConnections(NeuronBlock* neuronBlocks,
                         ConnectionBlock* connectionBlocks,
                         uint64_t* synapseBlockLinks,
                         SynapseBlock* synapseBlocks)
{
    // init global pointers
    NeuronBlock* targetNeuronBlock = &neuronBlocks[blockIdx.x];
    ConnectionBlock* connectionBlock = &connectionBlocks[blockIdx.x];
    SynapseBlock* synapseBlock = &synapseBlocks[synapseBlockLinks[blockIdx.x]];
    Connection* connection = &connectionBlock->connections[threadIdx.x];

    if (connection->origin.blockId != UNINIT_STATE_16 && connection->potential > 0.0f) {
        backpropagateNeuron(&synapseBlock->sections[threadIdx.x],
                            connection,
                            targetNeuronBlock);
    }
}

/**
 * @brief run backpropagaion on all normal- and output-brikcs to update the weights
 *        of the synapses.
 *
 * @param gpuPointer handle with all gpu-pointer of the cluster
 * @param hexagons pointer to local hexagons
 * @param numberOfHexagons number of hexagons
 * @param neuronBlocks pointer to local neuron-blocks
 * @param tempNeuronBlocks pointer to local temp-values of the neuron-blocks
 * @param numberOfNeuronBlocks number of neuron-blocks
 */
extern "C"
void
backpropagation_CUDA(Hexagon* hexagon,
                     SynapseBlock* synapseBlocks)
{
    cudaSetDevice(hexagon->cudaPointer.deviceId);

    if (hexagon->header.isInputHexagon) {
        return;
    }

    // copy necessary data from host to gpu
    cudaMemcpy(hexagon->cudaPointer.neuronBlocks,
               &hexagon->neuronBlocks[0],
               hexagon->neuronBlocks.size() * sizeof(NeuronBlock),
               cudaMemcpyHostToDevice);

    backpropagateNeurons<<<hexagon->header.numberOfBlocks, NEURONS_PER_NEURONBLOCK>>>(
            hexagon->cudaPointer.neuronBlocks);

    backpropagateConnections<<<hexagon->header.numberOfBlocks, NUMBER_OF_SYNAPSESECTION>>>(
            hexagon->cudaPointer.neuronBlocks,
            hexagon->cudaPointer.connectionBlocks,
            hexagon->cudaPointer.synapseBlockLinks,
            synapseBlocks);

    // copy neurons back to host
    cudaMemcpy(&hexagon->connectionBlocks[0],
               hexagon->cudaPointer.connectionBlocks,
               hexagon->connectionBlocks.size() * sizeof(ConnectionBlock),
               cudaMemcpyDeviceToHost);
}
