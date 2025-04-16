/**
 * @file        worker_thread.cpp
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

#include "worker_thread.h"

#include <processing/logical_host.h>
#include <src/cluster/cluster.h>
#include <unistd.h>

WorkerThread::WorkerThread() : Hanami::Thread("WorkerThread") {}

WorkerThread::~WorkerThread() {}

/**
 * @brief handle a task from the task-queue
 *
 * @param task task to handle
 */
void
WorkerThread::handleTask(const Hanami::WorkerTask& task)
{
    if (task.mode == ClusterProcessingMode::TRAIN_FORWARD_MODE) {
        handleTrainForwardTask(task);
    }
    else if (task.mode == ClusterProcessingMode::TRAIN_BACKWARD_MODE) {
        handleTrainBackwardTask(task);
    }
    else {
        handleProcessTask(task);
    }
}

/**
 * @brief rum worker-thread and get tasks for the task-queue of the connected cpu-host
 */
void
WorkerThread::run()
{
    while (m_abort == false) {
        Hanami::WorkerTask task = m_host->getWorkerTaskFromQueue();
        if (task.cluster != nullptr) {
            handleTask(task);
        }
        else {
            if (m_inactiveCounter < 100) {
                usleep(10);
                m_inactiveCounter++;
            }
            else {
                m_inactiveCounter = 0;
                blockThread();
            }
        }
    }
}
