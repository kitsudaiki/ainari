/**
 * @file        cluster_test.cpp
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

#include "cluster_test.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_init.h>
#include <core/cluster/objects.h>
#include <core/cluster/task.h>
#include <core/io/checkpoint/buffer/buffer_io.h>
#include <core/processing/logical_host.h>
#include <core/processing/physical_host.h>
#include <hanami_hardware/host.h>

namespace Hanami
{

Cluster_Test::Cluster_Test() : Hanami::MemoryLeakTestHelpter("Cluster_Test")
{
    initTest();
    initHost_test();
    createCluster_test();
    serialize_test();
    task_test();
}

/**
 * @brief initTest
 */
void
Cluster_Test::initTest()
{
    m_clusterTemplate
        = "version: 1\n"
          "settings:\n"
          "    refractory_time: 2\n"
          "    neuron_cooldown: 10000000.0\n"
          "    max_connection_distance: 3\n"
          "    enable_reduction: true\n"
          "\n"
          "hexagons:\n"
          "    1,1,1\n"
          "    3,1,1\n"
          "    4,1,1\n"
          "    \n"
          "axons:\n"
          "    1,1,1 -> 3,1,1\n"
          "\n"
          "inputs:\n"
          "    input_hexagon: 1,1,1\n"
          "\n"
          "outputs:\n"
          "    output_hexagon: 4,1,1\n"
          "\n";
}

/**
 * @brief initHost_test
 */
void
Cluster_Test::initHost_test()
{
    bool success = false;
    Hanami::ErrorContainer error;

    // init host
    PhysicalHost physicalHost;
    physicalHost.init(error);
    m_logicalHost = physicalHost.getFirstHost();
}

/**
 * @brief createCluster_test
 */
void
Cluster_Test::createCluster_test()
{
    const std::string uuid = generateUuid().toString();
    Hanami::ErrorContainer error;
    bool success = false;

    // parse template
    Hanami::ClusterMeta parsedCluster;
    Hanami::parseCluster(&parsedCluster, m_clusterTemplate, error);

    REINIT_TEST();

    // create new cluster
    Cluster* newCluster = new Cluster();
    newCluster->init(parsedCluster, uuid, m_logicalHost);
    delete newCluster;

    CHECK_MEMORY();
}

/**
 * @brief serialize_test
 */
void
Cluster_Test::serialize_test()
{
    const std::string uuid = generateUuid().toString();
    Hanami::ErrorContainer error;
    bool success = false;

    // parse template
    Hanami::ClusterMeta parsedCluster;
    success = Hanami::parseCluster(&parsedCluster, m_clusterTemplate, error);
    assert(success);

    REINIT_TEST();

    // create new cluster
    Cluster* baseCluster = new Cluster();
    assert(baseCluster->init(parsedCluster, uuid, m_logicalHost));

    // write cluster into a test-buffer
    BufferIO* bufferIo = new BufferIO();
    Hanami::DataBuffer* buffer = new Hanami::DataBuffer();
    bufferIo->writeClusterIntoBuffer(*buffer, *baseCluster, error);

    // read cluster from the test-buffer again
    Cluster* copyCluster = new Cluster();
    bufferIo->readClusterFromBuffer(*copyCluster, *buffer, m_logicalHost, error);

    delete baseCluster;
    delete bufferIo;
    delete copyCluster;
    delete buffer;

    CHECK_MEMORY();
}

/**
 * @brief task_test
 */
void
Cluster_Test::task_test()
{
    REINIT_TEST();

    Task* newTask = new Task();

    newTask->name = "asdf";
    newTask->userId = "asdf";
    newTask->projectId = "asdf";
    newTask->type = TRAIN_TASK;
    newTask->progress.queuedTimeStamp = std::chrono::system_clock::now();
    newTask->info = TrainInfo();
    TrainInfo* info = &std::get<TrainInfo>(newTask->info);
    info->timeLength = 42;

    Task* otherTask = new Task(std::move(*newTask));
    Task* otherTask2 = new Task();
    *otherTask2 = std::move(*otherTask);

    delete otherTask2;
    delete otherTask;
    delete newTask;

    CHECK_MEMORY();
}

}  // namespace Hanami
