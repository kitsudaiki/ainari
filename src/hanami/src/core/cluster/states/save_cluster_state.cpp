/**
 * @file        save_cluster_state.cpp
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

#include "save_cluster_state.h"

#include <config.h>
#include <core/cluster/cluster.h>
#include <core/cluster/statemachine_init.h>
#include <core/cluster/task.h>
#include <core/processing/cpu/cpu_host.h>
#include <core/processing/cuda/cuda_host.h>
#include <core/processing/logical_host.h>
#include <core/processing/physical_host.h>
#include <database/checkpoint_table.h>
#include <hanami_common/files/binary_file.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>

/**
 * @brief constructor
 *
 * @param cluster pointer to the cluster, where the event and the statemachine belongs to
 */
SaveCluster_State::SaveCluster_State(Cluster* cluster) { m_cluster = cluster; }

/**
 * @brief destructor
 */
SaveCluster_State::~SaveCluster_State() {}

/**
 * @brief prcess event
 *
 * @return true, if successful, else false
 */
bool
SaveCluster_State::processEvent()
{
    Hanami::ErrorContainer error;
    Task* currentTask = m_cluster->getCurrentTask();

    const bool success = saveClusterToCheckpoint(currentTask, error);

    m_cluster->goToNextState(FINISH_TASK);

    if (success == false) {
        error.addMessage("Failed to create checkpoint of cluster with UUID '" + m_cluster->getUuid()
                         + "'");
        // TODO: cleanup in error-case
        // TODO: give the user a feedback by setting the task to failed-state
    }

    return success;
}

/**
 * @brief store cluster into a checkpoint-file
 *
 * @param currentTask pointer to task
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
SaveCluster_State::saveClusterToCheckpoint(Task* currentTask, Hanami::ErrorContainer& error)
{
    // send checkpoint to shiori
    std::string fileUuid = "";
    // checkpoints are created by another internal process, which gives the id's not in the
    // context object, but as normal values
    Hanami::UserContext userContext;
    userContext.userId = currentTask->userId;
    userContext.projectId = currentTask->projectId;

    // get directory to store data from config
    bool success = false;
    std::filesystem::path targetFilePath
        = GET_STRING_CONFIG("storage", "checkpoint_location", success);
    if (success == false) {
        error.addMessage("checkpoint-location to store checkpoint is missing in the config");
        return false;
    }

    // build absolut file-path to store the file
    targetFilePath = targetFilePath
                     / std::filesystem::path(currentTask->uuid.toString() + "_checkpoint_"
                                             + currentTask->userId);

    const CheckpointSaveInfo* info = &std::get<CheckpointSaveInfo>(currentTask->info);

    // register in database
    CheckpointTable::CheckpointDbEntry checkpointData;
    checkpointData.uuid = currentTask->uuid.toString();
    checkpointData.name = info->checkpointName;
    checkpointData.location = targetFilePath.generic_string();
    checkpointData.projectId = currentTask->projectId;
    checkpointData.ownerId = currentTask->userId;
    checkpointData.visibility = "private";

    // add to database
    const ReturnStatus ret
        = CheckpointTable::getInstance()->addCheckpoint(checkpointData, userContext, error);
    if (ret == INVALID_INPUT) {
        return false;
    }
    if (ret == ERROR) {
        return false;
    }

    // write data of cluster to disc
    for (Hexagon& hexagon : m_cluster->hexagons) {
        hexagon.attachedHost->syncWithHost(&hexagon);
    }
    if (m_clusterIO.writeClusterToFile(*m_cluster, targetFilePath, error) != OK) {
        return false;
    }

    return true;
}
