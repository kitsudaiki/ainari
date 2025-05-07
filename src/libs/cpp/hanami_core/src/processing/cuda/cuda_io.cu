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

#include "error_handling.h"
#include "../../cluster/objects.h"

/**
 * @brief initDevice_CUDA
 * @param hostSynapseBlocks
 * @param numberOfSynapseBlocks
 * @return
 */
extern "C"
SynapseBlock*
initDevice_CUDA(SynapseBlock* hostSynapseBlocks,
                const uint32_t numberOfSynapseBlocks)
{
    SynapseBlock* deviceSynapseBlocks = nullptr;

    cudaMalloc(&deviceSynapseBlocks,
               numberOfSynapseBlocks * sizeof(SynapseBlock));
    cudaMemcpy(deviceSynapseBlocks,
               hostSynapseBlocks,
               numberOfSynapseBlocks * sizeof(SynapseBlock),
               cudaMemcpyHostToDevice);

    CHECK_LAST_CUDA_ERROR();

    return deviceSynapseBlocks;
}

/**
 * @brief initial copy of data from the host to the gpu
 *
 * @param gpuPointer pointer to the handle-object, which will store the pointer for the gpu-buffer
 * @param clusterSettings pointer to cluster-settings on host
 * @param neuronBlocks pointer to neuron-blocks on host
 * @param tempNeuronBlocks pointer to temp-values of the neuron-blocks on host
 * @param numberOfNeuronBlocks number of neuron-blocks to copy
 * @param synapseBlocks pointer to synapse-blocks on host
 * @param numberOfSynapseBlocks number of synapse-blocks to copy
 * @param hexagons pointer to hexagons to initialize their connection-blocks, if exist
 * @param numberOfHexagons number of hexagons in the cluster to init the connection-block-buffer
 */
extern "C"
void
initHexagonOnDevice_CUDA(Hexagon* hexagon,
                         ClusterSettings* clusterSettings,
                         SynapseBlock* hostSynapseBlocks,
                         SynapseBlock* deviceSynapseBlocks)
{
    cudaSetDevice(hexagon->cudaPointer.deviceId);

    // copy settings to gpu
    cudaMalloc(&hexagon->cudaPointer.clusterSettings, 1 * sizeof(ClusterSettings));
    cudaMemcpy(hexagon->cudaPointer.clusterSettings,
               clusterSettings,
               1 * sizeof(ClusterSettings),
               cudaMemcpyHostToDevice);

    if(hexagon->neuronBlocks.size() > 0) {
        cudaMalloc(&hexagon->cudaPointer.neuronBlocks,
                   hexagon->neuronBlocks.size() * sizeof(NeuronBlock));

        cudaMemcpy(hexagon->cudaPointer.neuronBlocks,
                   &hexagon->neuronBlocks[0],
                   hexagon->neuronBlocks.size() * sizeof(NeuronBlock),
                   cudaMemcpyHostToDevice);
    }

    if(hexagon->connectionBlocks.size() > 0) {
        cudaMalloc(&hexagon->cudaPointer.connectionBlocks,
                   hexagon->connectionBlocks.size() * sizeof(ConnectionBlock));
        cudaMemcpy(hexagon->cudaPointer.connectionBlocks,
                   &hexagon->connectionBlocks[0],
                   hexagon->connectionBlocks.size() * sizeof(ConnectionBlock),
                   cudaMemcpyHostToDevice);
    }

    if(hexagon->synapseBlockLinks.size() > 0) {
        cudaMalloc(&hexagon->cudaPointer.synapseBlockLinks,
                   hexagon->synapseBlockLinks.size() * sizeof(uint64_t));
        cudaMemcpy(hexagon->cudaPointer.synapseBlockLinks,
                   &hexagon->synapseBlockLinks[0],
                   hexagon->synapseBlockLinks.size() * sizeof(uint64_t),
                   cudaMemcpyHostToDevice);
    }

    for(const uint64_t link : hexagon->synapseBlockLinks) {
        cudaMemcpy(&deviceSynapseBlocks[link],
                   &hostSynapseBlocks[link],
                   sizeof(SynapseBlock),
                   cudaMemcpyHostToDevice);
    }
}

/**
 * @brief removed all data from the gpu, which are linked in the handle-object
 *
 * @param gpuPointer handle with all pointer to free
 */
extern "C"
void
removeFromDevice_CUDA(Hexagon* hexagon,
                      SynapseBlock* deviceSynapseBlocks)
{
    cudaSetDevice(hexagon->cudaPointer.deviceId);

    cudaFree(hexagon->cudaPointer.clusterSettings);

    if (hexagon->cudaPointer.neuronBlocks != nullptr)
    {
        cudaFree(hexagon->cudaPointer.neuronBlocks);
        hexagon->cudaPointer.neuronBlocks = nullptr;
    }

    if (hexagon->cudaPointer.connectionBlocks != nullptr)
    {
        cudaFree(hexagon->cudaPointer.connectionBlocks);
        hexagon->cudaPointer.connectionBlocks = nullptr;
    }

    if (hexagon->cudaPointer.synapseBlockLinks != nullptr)
    {
        cudaFree(hexagon->cudaPointer.synapseBlockLinks);
        hexagon->cudaPointer.synapseBlockLinks = nullptr;
    }

    for(const uint64_t link : hexagon->synapseBlockLinks) {
        cudaFree(&deviceSynapseBlocks[link]);
    }
}

/**
 * @brief copy all data from the gpu back to the host
 *
 * @param gpuPointer handle with all gpu-pointer of the cluster
 * @param neuronBlocks pointer to neuron-blocks on host
 * @param numberOfNeuronBlocks number of neuron-blocks to copy
 * @param synapseBlocks pointer to synpase-blocks on host
 * @param numberOfSynapseBlocks number of synpase-blocks to copy
 */
extern "C"
void
copyFromGpu_CUDA(Hexagon* hexagon,
                 SynapseBlock* hostSynapseBlocks,
                 SynapseBlock* deviceSynapseBlocks)
{
    cudaSetDevice(hexagon->cudaPointer.deviceId);

    cudaMemcpy(&hexagon->neuronBlocks[0],
               hexagon->cudaPointer.neuronBlocks,
               hexagon->neuronBlocks.size() * sizeof(NeuronBlock),
               cudaMemcpyDeviceToHost);

    cudaMemcpy(&hexagon->connectionBlocks[0],
               hexagon->cudaPointer.connectionBlocks,
               hexagon->connectionBlocks.size() * sizeof(ConnectionBlock),
               cudaMemcpyDeviceToHost);

    cudaMemcpy(&hexagon->synapseBlockLinks[0],
               hexagon->cudaPointer.synapseBlockLinks,
               hexagon->synapseBlockLinks.size() * sizeof(uint64_t),
               cudaMemcpyDeviceToHost);

    for(const uint64_t link : hexagon->synapseBlockLinks) {
        cudaMemcpy(&hostSynapseBlocks[link],
                   &deviceSynapseBlocks[link],
                   sizeof(SynapseBlock),
                   cudaMemcpyDeviceToHost);
    }
}

/**
 * @brief in case the cluster was resized, these changes have to be pushed to the gpu
 *
 * @param gpuPointer handle with all gpu-pointer of the cluster
 * @param neuronBlocks pointer to local buffer with neuron-blocks to update
 * @param numberOfNeuronBlocks number of neuron-blocks to update
 * @param hexagons pointer to local hexagons to access and update their connection-blocks
 * @param numberOfHexagons number of hexagons to update
 */
extern "C"
void
update_CUDA(Hexagon* hexagon,
            SynapseBlock* deviceSynapseBlocks)
{
    cudaSetDevice(hexagon->cudaPointer.deviceId);

    removeFromDevice_CUDA(hexagon, deviceSynapseBlocks);

    // allocate to resized memory for the connectionblocks on gpu
    cudaMalloc(&hexagon->cudaPointer.connectionBlocks,
               hexagon->connectionBlocks.size() * sizeof(ConnectionBlock));

    cudaMemcpy(hexagon->cudaPointer.connectionBlocks,
               &hexagon->connectionBlocks[0],
               hexagon->connectionBlocks.size() * sizeof(ConnectionBlock),
               cudaMemcpyHostToDevice);

    // allocate to resized memory for the neuronBlocks on gpu
    cudaMalloc(&hexagon->cudaPointer.neuronBlocks,
               hexagon->neuronBlocks.size() * sizeof(ConnectionBlock));

    cudaMemcpy(hexagon->cudaPointer.neuronBlocks,
               &hexagon->neuronBlocks[0],
               hexagon->neuronBlocks.size() * sizeof(ConnectionBlock),
               cudaMemcpyHostToDevice);

    // allocate to resized memory for the synapseBlockLinks on gpu
    cudaMalloc(&hexagon->cudaPointer.synapseBlockLinks,
               hexagon->synapseBlockLinks.size() * sizeof(ConnectionBlock));

    cudaMemcpy(hexagon->cudaPointer.synapseBlockLinks,
               &hexagon->synapseBlockLinks[0],
               hexagon->synapseBlockLinks.size() * sizeof(ConnectionBlock),
               cudaMemcpyHostToDevice);

    hexagon->wasResized = false;
}
