/**
 * @file        logical_host.cpp
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

#include "logical_host.h"

#include <api/websocket/cluster_io.h>
#include <core/cluster/cluster.h>
#include <core/cluster/cluster_io_convert.h>
#include <core/cluster/objects.h>
#include <core/processing/cpu/core_processing.h>
#include <hanami_common/buffer/item_buffer.h>

/**
 * @brief constructor
 *
 * @param localId identifier starting with 0 within the physical host and with the type of host
 */
LogicalHost::LogicalHost(const uint32_t localId)
{
    m_localId = localId;
    m_uuid = generateUuid().toString();
}

/**
 * @brief destructor
 */
LogicalHost::~LogicalHost() {}

/**
 * @brief get host-type of this logical host (cpu, cuda, ...)
 */
LogicalHost::HostType
LogicalHost::getHostType() const
{
    return m_hostType;
}

/**
 * @brief get uuid of the host for identification
 */
const std::string
LogicalHost::getUuid() const
{
    return m_uuid;
}

/**
 * @brief get amount of initialy defined available memory
 */
uint64_t
LogicalHost::getTotalMemory()
{
    return m_totalMemory;
}

/**
 * @brief re-activate all blocked threads
 */
void
LogicalHost::continueAllThreads()
{
    for (WorkerThread* worker : m_workerThreads) {
        worker->continueThread();
    }
}

/**
 * @brief get next cluster in the queue
 *
 * @return nullptr, if queue is empty, else next cluster in queue
 */
const Hanami::WorkerTask
LogicalHost::getWorkerTaskFromQueue()
{
    Hanami::WorkerTask result;
    const std::lock_guard<std::mutex> lock(m_queue_lock);

    if (m_workerTaskQueue.size() > 0) {
        result = m_workerTaskQueue.front();
        m_workerTaskQueue.pop_front();
    }

    return result;
}

/**
 * @brief add cluster to queue
 *
 * @param cluster cluster to add to queue
 */
void
LogicalHost::addWorkerTaskToQueue(const Hanami::WorkerTask task)
{
    const std::lock_guard<std::mutex> lock(m_queue_lock);

    m_workerTaskQueue.push_back(task);
    continueAllThreads();
}

/**
 * @brief in case of a request-task the output is either written into a request-result in case of a
 *        task or is send via websocket to a client in case of direct-io
 *
 * @param cluster cluster to handle
 */
void
handleClientOutput(Cluster* cluster)
{
    Hanami::ErrorContainer error;
    // send output back if a client-connection is set

    Task* actualTask = cluster->getCurrentTask();
    if (actualTask != nullptr && actualTask->type == REQUEST_TASK) {
        RequestInfo* info = &std::get<RequestInfo>(actualTask->info);

        for (auto& [name, outputInterface] : cluster->outputInterfaces) {
            DataSetFileHandle* fileHandle = &info->results[name];
            const uint64_t ioBufferSize = convertOutputToBuffer(&outputInterface);
            // TODO: handle return status
            appendToDataSet(
                *fileHandle, &outputInterface.ioBuffer[0], ioBufferSize * sizeof(float), error);
        }
    }
    if (cluster->msgClient != nullptr) {
        // TODO: handle return status
        sendClusterOutputMessage(cluster);
    }
}
