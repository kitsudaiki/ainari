/**
 * @file        train_forward_state.cpp
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

#include "train_forward_state.h"

#include <cluster/cluster.h>
#include <cluster/cluster_io_convert.h>

/**
 * @brief constructor
 *
 * @param cluster pointer to the cluster, where the event and the statemachine belongs to
 */
TrainForward_State::TrainForward_State(Cluster* cluster) { m_cluster = cluster; }

/**
 * @brief destructor
 */
TrainForward_State::~TrainForward_State() {}

/**
 * @brief prcess event
 *
 * @return alway true
 */
bool
TrainForward_State::processEvent()
{
    Hanami::ErrorContainer error;
    Task* actualTask = m_cluster->getCurrentTask();
    TrainInfo* info = &std::get<TrainInfo>(actualTask->info);

    AxonBlock* axonBlock = nullptr;
    Axon* axon = nullptr;
    uint64_t blockId = 0;
    uint16_t axonId = 0;

    for (auto& [hexagonName, input] : info->inputs) {
        uint64_t counter = 0;
        InputInterface* inputInterface = &m_cluster->inputInterfaces[hexagonName];

        for (uint64_t t = 0; t < info->timeLength; ++t) {
            if (getDataFromDataSet(inputInterface->ioBuffer, input, info->currentCycle + t, error)
                != OK)
            {
                return false;
            }
            for (const float val : inputInterface->ioBuffer) {
                blockId = counter / NEURONS_PER_BLOCK;
                axonId = counter % NEURONS_PER_BLOCK;
                axonBlock = &inputInterface->inputAxons[blockId];
                axonBlock->axons[axonId].potential = 0.0f;
                axonBlock->axons[axonId + 1].potential = 0.0f;
                axon = &axonBlock->axons[axonId + (val >= 0.0f)];
                axon->potential = abs(val);
                counter += 2;
            }
        }
    }

    for (auto& [hexagonName, output] : info->outputs) {
        OutputInterface* outputInterface = &m_cluster->outputInterfaces[hexagonName];
        if (getDataFromDataSet(outputInterface->ioBuffer,
                               output,
                               (info->timeLength - 1) + info->currentCycle,
                               error)
            != OK)
        {
            return false;
        }
        convertBufferToExpected(outputInterface);
    }

    m_cluster->startForwardCycle(false);

    return true;
}
