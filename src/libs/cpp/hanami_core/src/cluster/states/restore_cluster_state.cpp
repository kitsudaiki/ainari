/**
 * @file        restore_cluster_state.cpp
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

#include "restore_cluster_state.h"

#include <cluster/cluster.h>
#include <cluster/cluster_init.h>
#include <cluster/statemachine_init.h>
#include <cluster/task.h>
#include <hanami_common/files/binary_file.h>
#include <hanami_common/functions/common_functions.h>
#include <include/hanami_core/hanami_root.h>
#include <processing/cpu/cpu_host.h>
#include <processing/logical_host.h>
#include <processing/physical_host.h>

/**
 * @brief constructor
 *
 * @param cluster pointer to the cluster, where the event and the statemachine belongs to
 */
RestoreCluster_State::RestoreCluster_State(Cluster* cluster) { m_cluster = cluster; }

/**
 * @brief destructor
 */
RestoreCluster_State::~RestoreCluster_State() {}

/**
 * @brief prcess event
 *
 * @return true, if successful, else false
 */
bool
RestoreCluster_State::processEvent()
{
    Task* currentTask = m_cluster->getCurrentTask();
    Hanami::ErrorContainer error;

    const bool success = restoreClusterFromCheckpoint(currentTask, error);

    m_cluster->goToNextState(FINISH_TASK);

    return success;
}

/**
 * @brief restore cluster from a checkpoint-file
 *
 * @param currentTask pointer to task
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
RestoreCluster_State::restoreClusterFromCheckpoint(Task* currentTask, Hanami::ErrorContainer& error)
{
    const CheckpointRestoreInfo* info = &std::get<CheckpointRestoreInfo>(currentTask->info);
    const std::string location = info->checkpointInfo.location;
    const ReturnStatus ret
        = m_clusterIO.restoreClusterFromFile(*m_cluster, location, nullptr, error);
    if (ret != OK) {
        return false;
    }

    return true;
}
