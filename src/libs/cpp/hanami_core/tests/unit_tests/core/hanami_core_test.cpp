/**
 * @file        hanami_core_test.cpp
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

#include "hanami_core_test.h"

#include "hanami_root.h"

Hanami_Core_Test::Hanami_Core_Test() : Hanami::CompareTestHelper("Cluster_Init_Test")
{
    initTest();

    core_test();
}

void
Hanami_Core_Test::initTest()
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

void
Hanami_Core_Test::core_test()
{
    HanamiCore core;
    const std::string uuid = "ce8eaaf9-8798-42d6-b8a1-5ddbe073178a";
    std::string errorMessage = "";

    bool ret = core.init(0.01f, errorMessage);
    TEST_EQUAL(ret, true);
    ret = core.init(0.01f, errorMessage);
    TEST_EQUAL(ret, false);

    // create
    // ret = core.createCluster(uuid, "test-cluster", m_clusterTemplate, errorMessage);
    // TEST_EQUAL(ret, OK);
    // ret = core.createCluster(uuid, "test-cluster", m_clusterTemplate, errorMessage);
    // TEST_EQUAL(ret, INVALID_INPUT);

    // delete
    // ret = core.deleteCluster(uuid);
    // TEST_EQUAL(ret, OK);
    // ret = core.deleteCluster(uuid);
    // TEST_EQUAL(ret, INVALID_INPUT);
}
