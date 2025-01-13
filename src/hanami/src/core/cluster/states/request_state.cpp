/**
 * @file        request_state.cpp
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

#include "request_state.h"

#include <core/cluster/cluster.h>

/**
 * @brief constructor
 *
 * @param cluster pointer to the cluster, where the event and the statemachine belongs to
 */
Request_State::Request_State(Cluster* cluster) { m_cluster = cluster; }

/**
 * @brief destructor
 */
Request_State::~Request_State() {}

/**
 * @brief prcess event
 *
 * @return alway true
 */
bool
Request_State::processEvent()
{
    Hanami::ErrorContainer error;
    Task* actualTask = m_cluster->getCurrentTask();
    RequestInfo* info = &std::get<RequestInfo>(actualTask->info);

    for (auto& [hexagonName, input] : info->inputs) {
        uint64_t counter = 0;
        InputInterface* inputInterface = &m_cluster->inputInterfaces[hexagonName];
        AxonBlock* axonBlock = nullptr;
        for (uint64_t t = 0; t < info->timeLength; ++t) {
            if (getDataFromDataSet(inputInterface->ioBuffer, input, info->currentCycle + t, error)
                != OK)
            {
                return false;
            }
            for (const float val : inputInterface->ioBuffer) {
                axonBlock = &inputInterface->inputAxons[counter / NEURONS_PER_BLOCK];
                axonBlock->axons[counter % NEURONS_PER_BLOCK].potential = val;
                counter++;
            }
        }
    }

    m_cluster->startForwardCycle(true);

    return true;
}
