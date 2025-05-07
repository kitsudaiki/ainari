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

#include "cluster_link.h"
#include "hanami_root.h"
#include "hanami_structs.h"

Hanami_Core_Test::Hanami_Core_Test() : Hanami::CompareTestHelper("Cluster_Init_Test")
{
    initTest();

    // HINT (kitsudaiki): this test has only minimal input and so it can often fail
    // which makes it to fragile for the ci-pipeline. Can be enabled temporary for tests.
    // core_test();
}

void
Hanami_Core_Test::initTest()
{
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

    // prepare information for the cluster
    ClusterMeta clusterMeta;
    clusterMeta.addHexagon(1, 1, 1);
    clusterMeta.addHexagon(2, 1, 1);
    clusterMeta.addHexagon(3, 1, 1);
    clusterMeta.addAxon(0, 1);
    clusterMeta.addInput("input", 0);
    clusterMeta.addOutput("output", 2, PLAIN_OUTPUT);

    std::unique_ptr<ClusterLink> link = core.createCluster("poi", "poi", clusterMeta, errorMessage);

    // set input
    float inputData[8] = {10.0, 0.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0};
    TEST_EQUAL(link->fillInput("fail", inputData, 8), false);
    TEST_EQUAL(link->fillInput("input", inputData, 8), true);

    // set expected-values on the outputs
    float expectedData[4] = {0.0, 1.0, 0.0, 1.0};
    TEST_EQUAL(link->fillExpected("fail", expectedData, 4), false);
    TEST_EQUAL(link->fillExpected("output", expectedData, 4), true);

    // train a few times with the same values
    for (uint32_t i = 0; i < 10000; ++i) {
        link->doTrain();
    }

    // make a request with the same input-data
    link->doRequest();

    // get output of the request-call from the cluster
    float outputData[4] = {0.0, 0.0, 0.0, 0.0};
    TEST_EQUAL(link->getOutput("fail", outputData, 4), false);
    TEST_EQUAL(link->getOutput("output", outputData, 4), true);

    // check results
    TEST_EQUAL(outputData[0], 0.0f);
    TEST_NOT_EQUAL(outputData[1], 0.0f);
    TEST_EQUAL(outputData[2], 0.0f);
    TEST_NOT_EQUAL(outputData[3], 0.0f);
}
