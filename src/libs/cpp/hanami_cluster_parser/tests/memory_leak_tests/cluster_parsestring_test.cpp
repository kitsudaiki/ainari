/**
 * @file       cluster_parsestring_test.cpp
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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

#include "cluster_parsestring_test.h"

#include <hanami_cluster_parser/cluster_meta.h>

namespace Hanami
{

Cluster_ParseString_Test::Cluster_ParseString_Test()
    : Hanami::MemoryLeakTestHelpter("Cluster_ParseString_Test")
{
    parseString_test();
}

/**
 * parseString_test
 */
void
Cluster_ParseString_Test::parseString_test()
{
    const std::string input(
        "version: 1\n"
        "settings:\n"
        "    refractory_time: 1\n"
        "    neuron_cooldown: 10000000.0\n"
        "    max_connection_distance: 1\n"
        "\n"
        "hexagons:\n"
        "    1,1,1\n"
        "    2,1,1\n"
        "    4,1,1\n"
        "    5,1,1\n"
        "\n"
        "axons:\n"
        "    1,1,1 -> 4,1,1\n"
        "    2,1,1 -> 4,1,1\n"
        "\n"
        "inputs:\n"
        "    input_hexagon: 1,1,1\n"
        "    input_hexagon2: 2,1,1\n"
        "\n"
        "outputs:\n"
        "    output_hexagon: 5,1,1\n"
        "\n");

    ClusterMeta result;
    ErrorContainer error;

    // the parser-interface is a singleton, which is initialized on the first usage
    // so a unchecked run has to be done, for the initializing
    parseCluster(&result, input, error);

    REINIT_TEST();
    parseCluster(&result, input, error);
    CHECK_MEMORY();
}

}  // namespace Hanami
