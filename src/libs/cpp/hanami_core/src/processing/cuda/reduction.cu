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
 * @brief backpropagate a synapse-section
 *
 * @param section current synapse-section
 */
__device__ __forceinline__ bool
reduceSection(SynapseSection* section)
{
    Synapse* synapse;
    uint8_t exist = 0;

    for (uint8_t pos = 0; pos < SYNAPSES_PER_SYNAPSESECTION; pos++) {
        synapse = &section->synapses[pos];

        if (synapse->targetNeuronId != UNINIT_STATE_8) {
            synapse->activeCounter -= static_cast<uint8_t>(synapse->activeCounter < 10);

            // handle active-counter
            if (synapse->activeCounter == 0) {
                if (pos < SYNAPSES_PER_SYNAPSESECTION - 1) {
                    section->synapses[pos] = section->synapses[pos + 1];
                    section->synapses[pos + 1] = Synapse();
                } else {
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
 * @brief reduce synapse, in order to limit the amount of memory
 *
 * @param connectionBlocks pointer to connection-blocks
 * @param neuronBlocks pointer to neuron-blocks
 * @param synapseBlocks pointer to synapse-blocks
 */
__global__ void
reduceConnections(ConnectionBlock* connectionBlocks,
                  NeuronBlock* neuronBlocks,
                  SynapseBlock* synapseBlocks)
{
    Neuron* sourceNeuron = nullptr;
    NeuronBlock* sourceNeuronBlock = nullptr;
    SynapseSection* synapseSection = nullptr;

    ConnectionBlock* connectionBlock = &connectionBlocks[blockIdx.x];
    SynapseConnection* connection = &connectionBlock->connections[threadIdx.x];

    if (connection->origin.blockId != UNINIT_STATE_16) {
        synapseSection = &synapseBlocks[connectionBlock->targetSynapseBlockPos].sections[threadIdx.x];
        sourceNeuronBlock = &neuronBlocks[connection->origin.blockId];
        sourceNeuron = &sourceNeuronBlock->neurons[connection->origin.neuronId];

        // if section is complete empty, then erase it
        if (reduceSection(synapseSection) == false) {
            // initialize the creation of a new section
            sourceNeuron->isNew = 1;
            sourceNeuron->newLowerBound = connection->lowerBound;

            // mark current connection as available again
            //connection->origin.blockId = UNINIT_STATE_32;
            connection->origin.neuronId = UNINIT_STATE_8;
        }
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
 * @param numberOfNeuronBlocks number of neuron-blocks
 */
extern "C"
void
reduction_CUDA(CudaClusterPointer* gpuPointer,
               std::vector<Hexagon>& hexagons)
{
    cudaSetDevice(gpuPointer->deviceId);


    // process all hexagons on gpu
    for (int32_t hexagonId = hexagons.size() - 1; hexagonId >= 0; --hexagonId)
    {
        Hexagon* hexagon = &hexagons[hexagonId];
        if (hexagon->header.isInputHexagon) {
            continue;
        }

        // copy necessary data from host to gpu
        cudaMemcpy(gpuPointer->hexagonPointer[hexagonId].neuronBlocks,
                   &hexagon->neuronBlocks[0],
                   hexagon->neuronBlocks.size() * sizeof(NeuronBlock),
                   cudaMemcpyHostToDevice);

        reduceConnections<<<hexagon->header.dimX, 64>>>(
                gpuPointer->hexagonPointer[hexagonId].connectionBlocks,
                gpuPointer->hexagonPointer[hexagonId].neuronBlocks,
                gpuPointer->synapseBlocks);


        cudaMemcpy(&hexagon->connectionBlocks[0],
                   gpuPointer->hexagonPointer[hexagonId].connectionBlocks,
                   hexagon->connectionBlocks.size() * sizeof(ConnectionBlock),
                   cudaMemcpyDeviceToHost);

        // copy neurons back to host
        cudaMemcpy(&hexagon->neuronBlocks[0],
                   gpuPointer->hexagonPointer[hexagonId].neuronBlocks,
                   hexagon->neuronBlocks.size() * sizeof(NeuronBlock),
                   cudaMemcpyDeviceToHost);
    }

}
