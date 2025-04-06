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

#include <cluster/cluster.h>
#include <cluster/cluster_init.h>
#include <cluster/objects.h>
#include <hanami_common/logger.h>
#include <hanami_hardware/host.h>
#include <io/checkpoint/buffer/buffer_io.h>
#include <processing/logical_host.h>
#include <processing/physical_host.h>

Cluster_Init_Test::Cluster_Init_Test() : Hanami::CompareTestHelper("Cluster_Init_Test")
{
    initTest();
    initHost_test();
    createCluster_test();
    serialize_test();
}

/**
 * @brief initTest
 */
void
Cluster_Init_Test::initTest()
{
    m_clusterTemplate
        = "version: 1\n"
          "settings:\n"
          "    refractory_time: 2\n"
          "    neuron_cooldown: 10000000.0\n"
          "    max_connection_distance: 3\n"
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
Cluster_Init_Test::initHost_test()
{
    bool success = false;
    Hanami::ErrorContainer error;

    // init host
    PhysicalHost physicalHost(0.001f);
    physicalHost.init(error);
    m_logicalHost = physicalHost.getFirstHost();
    success = m_logicalHost != nullptr;
    TEST_EQUAL(success, true);
}

/**
 * @brief createCluster_test
 */
void
Cluster_Init_Test::createCluster_test()
{
    const std::string uuid = "ce8eaaf9-8798-42d6-b8a1-5ddbe073178a";
    Hanami::ErrorContainer error;
    bool success = false;

    // parse template
    Hanami::ClusterMeta parsedCluster;
    success = Hanami::parseCluster(&parsedCluster, m_clusterTemplate, error);
    TEST_EQUAL(success, true);
    if (success == false) {
        LOG_ERROR(error);
    }

    // create new cluster
    Cluster newCluster;
    success = newCluster.init(parsedCluster, uuid, m_logicalHost);
    TEST_EQUAL(success, true);

    // test uuid
    TEST_EQUAL(newCluster.getUuid(), uuid);

    // test settings
    TEST_EQUAL(newCluster.clusterHeader.settings.neuronCooldown, 10000000.0);
    TEST_EQUAL(newCluster.clusterHeader.settings.refractoryTime, 2);
    TEST_EQUAL(newCluster.clusterHeader.settings.maxConnectionDistance, 3);

    // test hexagons
    TEST_EQUAL(newCluster.hexagons.size(), 3);
    TEST_EQUAL(newCluster.hexagons.at(0).header.hexagonId, 0);
    TEST_EQUAL(newCluster.hexagons.at(1).header.hexagonId, 1);
    TEST_EQUAL(newCluster.hexagons.at(2).header.hexagonId, 2);
    TEST_EQUAL(newCluster.hexagons.at(0).header.isInputHexagon, true);
    TEST_EQUAL(newCluster.hexagons.at(0).header.isOutputHexagon, false);
    TEST_EQUAL(newCluster.hexagons.at(1).header.isInputHexagon, false);
    TEST_EQUAL(newCluster.hexagons.at(1).header.isOutputHexagon, false);
    TEST_EQUAL(newCluster.hexagons.at(2).header.isInputHexagon, false);
    TEST_EQUAL(newCluster.hexagons.at(2).header.isOutputHexagon, true);

    // test neighbors of hexagon 0
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[0], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[1], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[2], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[3], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[4], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[5], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[6], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[7], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[8], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[9], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[10], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).neighbors[11], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(0).header.axonTarget, 1);
    success = newCluster.hexagons.at(0).inputInterface != nullptr;
    TEST_EQUAL(success, true);
    success = newCluster.hexagons.at(0).outputInterface == nullptr;
    TEST_EQUAL(success, true);
    success = true;
    for (uint32_t i = 0; i < NUMBER_OF_POSSIBLE_NEXT; i++) {
        success &= newCluster.hexagons.at(0).possibleHexagonTargetIds[i] < 3
                   && newCluster.hexagons.at(0).possibleHexagonTargetIds[i] != 0;
    }
    TEST_EQUAL(success, true);

    // test neighbors of hexagon 1
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[0], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[1], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[2], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[3], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[4], 2);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[5], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[6], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[7], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[8], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[9], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[10], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).neighbors[11], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(1).header.axonTarget, 1);
    success = newCluster.hexagons.at(1).inputInterface == nullptr;
    TEST_EQUAL(success, true);
    success = newCluster.hexagons.at(1).outputInterface == nullptr;
    TEST_EQUAL(success, true);
    success = true;
    for (uint32_t i = 0; i < NUMBER_OF_POSSIBLE_NEXT; i++) {
        success &= newCluster.hexagons.at(1).possibleHexagonTargetIds[i] < 3
                   && newCluster.hexagons.at(1).possibleHexagonTargetIds[i] != 1;
    }
    TEST_EQUAL(success, true);

    // test neighbors of hexagon 2
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[0], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[1], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[2], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[3], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[4], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[5], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[6], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[7], 1);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[8], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[9], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[10], UNINIT_STATE_32);
    TEST_EQUAL(newCluster.hexagons.at(2).neighbors[11], UNINIT_STATE_32);
    success = newCluster.hexagons.at(2).inputInterface == nullptr;
    TEST_EQUAL(success, true);
    success = newCluster.hexagons.at(2).outputInterface != nullptr;
    TEST_EQUAL(success, true);
    success = true;
    for (uint32_t i = 0; i < NUMBER_OF_POSSIBLE_NEXT; i++) {
        success &= newCluster.hexagons.at(2).possibleHexagonTargetIds[i] == UNINIT_STATE_32;
    }
    TEST_EQUAL(success, true);

    // test input-interfaces
    TEST_EQUAL(newCluster.inputInterfaces.size(), 1);
    TEST_EQUAL(newCluster.inputInterfaces.begin()->second.targetHexagonId, 0);

    // test output-interfaces
    TEST_EQUAL(newCluster.outputInterfaces.size(), 1);
    TEST_EQUAL(newCluster.outputInterfaces.begin()->second.targetHexagonId, 2);
}

/**
 * @brief serialize_test
 */
void
Cluster_Init_Test::serialize_test()
{
    const std::string uuid = "ce8eaaf9-8798-42d6-b8a1-5ddbe073178a";
    Hanami::ErrorContainer error;
    bool success = false;

    // parse template
    Hanami::ClusterMeta parsedCluster;
    success = Hanami::parseCluster(&parsedCluster, m_clusterTemplate, error);
    assert(success);

    // create new cluster
    Cluster baseCluster;
    assert(baseCluster.init(parsedCluster, uuid, m_logicalHost));

    // write cluster into a test-buffer
    BufferIO bufferIo;
    Hanami::DataBuffer buffer;
    TEST_EQUAL(bufferIo.writeClusterIntoBuffer(buffer, baseCluster, error), OK);

    // read cluster from the test-buffer again
    Cluster copyCluster;
    TEST_EQUAL(bufferIo.readClusterFromBuffer(copyCluster, buffer, m_logicalHost, error), OK);

    // check cluster itself
    success = copyCluster.clusterHeader == baseCluster.clusterHeader;
    TEST_EQUAL(success, true);
    TEST_EQUAL(copyCluster.hexagons.size(), baseCluster.hexagons.size());
    TEST_EQUAL(copyCluster.inputInterfaces.size(), baseCluster.inputInterfaces.size());
    TEST_EQUAL(copyCluster.outputInterfaces.size(), baseCluster.outputInterfaces.size());

    // check hexagons
    for (uint64_t i = 0; i < copyCluster.hexagons.size(); i++) {
        success = copyCluster.hexagons[i].header == baseCluster.hexagons[i].header;
        TEST_EQUAL(success, true);
        TEST_EQUAL(copyCluster.hexagons[i].blockLinks.size(),
                   baseCluster.hexagons[i].blockLinks.size());
    }
}
