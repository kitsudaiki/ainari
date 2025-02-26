/**
 * @file        cuda_worker_thread.cpp
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

#include "cuda_worker_thread.h"

#include <core/cluster/objects.h>
#include <core/processing/cluster_resize.h>
#include <core/processing/cpu/core_backpropagation.h>
#include <core/processing/cpu/core_processing.h>
#include <core/processing/cpu/output_backpropagation.h>
#include <core/processing/cpu/output_processing.h>
#include <core/processing/cuda/cuda_functions.h>
#include <core/processing/logical_host.h>

/**
 * @brief constructor
 *
 * @param host pointer to related cuda-host, which holds the task-queue for the worker
 */
CudaWorkerThread::CudaWorkerThread(CudaHost* host) : WorkerThread()
{
    m_host = host;
    m_cudaHost = host;
}

/**
 * @brief destructor
 */
CudaWorkerThread::~CudaWorkerThread() {}

/**
 * @brief handle trainging task
 *
 * @param task task to handle
 */
void
CudaWorkerThread::handleTrainForwardTask(Hanami::WorkerTask task)
{
    Hexagon* hexagon = &task.cluster->hexagons[task.hexagonId];

    // handle special-case that there are no neuron-blocks to process
    if (hexagon->blockLinks.size() == 0) {
        // in case of the last hexagon
        if (task.hexagonId == task.cluster->hexagons.size() - 1) {
            task.cluster->updateClusterState(task);
            return;
        }

        // in case of a normal hexagon
        Hanami::WorkerTask newTask;
        newTask.cluster = task.cluster;
        newTask.hexagonId = task.hexagonId + 1;
        newTask.blockId = UNINIT_STATE_16;
        newTask.mode = task.mode;
        task.cluster->hexagons[newTask.hexagonId].attachedHost->addWorkerTaskToQueue(newTask);
        return;
    }

    // processing_CUDA(hexagon, m_cudaHost->deviceBlocks, true);

    if (task.cluster->incrementAndCompare(task.cluster->hexagons[task.hexagonId].blockLinks.size()))
    {
        if (hexagon->outputInterface != nullptr) {
            processNeuronsOfOutputHexagon<true>(hexagon, rand());
        }

        if (task.hexagonId == task.cluster->hexagons.size() - 1) {
            updateCluster(task.cluster, hexagon);
            task.cluster->updateClusterState(task);
        }
        else {
            Hanami::WorkerTask newTask;
            newTask.cluster = task.cluster;
            newTask.hexagonId = task.hexagonId + 1;
            newTask.blockId = UNINIT_STATE_16;
            newTask.mode = task.mode;
            task.cluster->hexagons[newTask.hexagonId].attachedHost->addWorkerTaskToQueue(newTask);
        }
    }
}

/**
 * @brief handle backpropagation task
 *
 * @param task task to handle
 */
void
CudaWorkerThread::handleTrainBackwardTask(Hanami::WorkerTask task)
{
    Hexagon* hexagon = &task.cluster->hexagons[task.hexagonId];

    // handle output-interface
    if (hexagon->outputInterface != nullptr) {
        backpropagateOutput(hexagon->outputInterface);
    }

    // handle special-case that there are no neuron-blocks to process
    if (hexagon->blockLinks.size() == 0) {
        if (task.hexagonId == 0) {
            task.cluster->updateClusterState(task);
            return;
        }

        Hanami::WorkerTask newTask;
        newTask.cluster = task.cluster;
        newTask.hexagonId = task.hexagonId - 1;
        newTask.blockId = UNINIT_STATE_16;
        newTask.mode = task.mode;
        task.cluster->hexagons[newTask.hexagonId].attachedHost->addWorkerTaskToQueue(newTask);
        return;
    }

    // backpropagation_CUDA(hexagon, m_cudaHost->deviceBlocks);

    if (task.cluster->incrementAndCompare(task.cluster->hexagons[task.hexagonId].blockLinks.size()))
    {
        if (task.hexagonId == 0) {
            task.cluster->updateClusterState(task);
        }
        else {
            Hanami::WorkerTask newTask;
            newTask.cluster = task.cluster;
            newTask.hexagonId = task.hexagonId - 1;
            newTask.blockId = UNINIT_STATE_16;
            newTask.mode = task.mode;
            task.cluster->hexagons[newTask.hexagonId].attachedHost->addWorkerTaskToQueue(newTask);
        }
    }
}

/**
 * @brief handle process task
 *
 * @param task task to handle
 */
void
CudaWorkerThread::handleProcessTask(const Hanami::WorkerTask task)
{
    Hexagon* hexagon = &task.cluster->hexagons[task.hexagonId];

    // handle special-case that there are no neuron-blocks to process
    if (hexagon->blockLinks.size() == 0) {
        if (task.hexagonId == task.cluster->hexagons.size() - 1) {
            task.cluster->updateClusterState(task);
            return;
        }

        Hanami::WorkerTask newTask;
        newTask.cluster = task.cluster;
        newTask.hexagonId = task.hexagonId + 1;
        newTask.blockId = UNINIT_STATE_16;
        newTask.mode = task.mode;
        task.cluster->hexagons[newTask.hexagonId].attachedHost->addWorkerTaskToQueue(newTask);
        return;
    }

    // processing_CUDA(hexagon, m_cudaHost->deviceBlocks, false);

    if (task.cluster->incrementAndCompare(task.cluster->hexagons[task.hexagonId].blockLinks.size()))
    {
        if (hexagon->outputInterface != nullptr) {
            processNeuronsOfOutputHexagon<false>(hexagon, rand());
        }

        if (task.hexagonId == task.cluster->hexagons.size() - 1) {
            handleClientOutput(task.cluster);
            task.cluster->updateClusterState(task);
        }
        else {
            Hanami::WorkerTask newTask;
            newTask.cluster = task.cluster;
            newTask.hexagonId = task.hexagonId + 1;
            newTask.blockId = UNINIT_STATE_16;
            newTask.mode = task.mode;
            task.cluster->hexagons[newTask.hexagonId].attachedHost->addWorkerTaskToQueue(newTask);
        }
    }
}
