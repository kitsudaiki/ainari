/**
 * @file        task_handle_state.cpp
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

#include "task_handle_state.h"

#include <cluster/cluster.h>
#include <cluster/statemachine_init.h>
#include <include/hanami_core/hanami_root.h>

/**
 * @brief constructor
 *
 * @param cluster pointer to the cluster, where the event and the statemachine belongs to
 */
TaskHandle_State::TaskHandle_State(Cluster* cluster) { m_cluster = cluster; }

/**
 * @brief destructor
 */
TaskHandle_State::~TaskHandle_State() {}

/**
 * @brief prcess event
 *
 * @return false, if statechange failed, else true
 */
bool
TaskHandle_State::processEvent()
{
    Hanami::ErrorContainer error;
    const TaskType nextTaskType = m_cluster->finishTask();
    switch (nextTaskType) {
        case NO_TASK:
        {
            return true;
        }
        case TRAIN_TASK:
        {
            if (m_cluster->goToNextState(TRAIN)) {
                return true;
            }
            else {
                // TODO: error-message
                return false;
            }
            break;
        }
        case REQUEST_TASK:
        {
            if (m_cluster->goToNextState(REQUEST)) {
                return true;
            }
            else {
                // TODO: error-message
                return false;
            }
            break;
        }
        default:
        {
            // TODO: error-message
            return false;
        }
    }

    return true;
}
