/**
 * @file        show_task_v1_0.cpp
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

#include "show_task_v1_0.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_handler.h>
#include <database/cluster_table.h>
#include <hanami_root.h>

ShowTaskV1M0::ShowTaskV1M0() : Blossom("Show information of a specific task.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the cluster, which should process the request")
        .setRegex(UUID_REGEX);

    registerInputField("cluster_uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the cluster, which should process the request")
        .setRegex(UUID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the task.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the task.");

    registerOutputField("current_cycle", SAKURA_INT_TYPE)
        .setComment("Current cycle of the current task.");

    registerOutputField("total_number_of_cycles", SAKURA_INT_TYPE)
        .setComment("Total number of cycles requred by the task.");

    registerOutputField("state", SAKURA_STRING_TYPE)
        .setComment("Actual state of the task (queued, active, aborted or finished).");

    registerOutputField("queue_timestamp", SAKURA_STRING_TYPE)
        .setComment(
            "Timestamp in UTC when the task entered the queued state, "
            "which is basicall the timestamp when the task was created");

    registerOutputField("start_timestamp", SAKURA_STRING_TYPE)
        .setComment("Timestamp in UTC when the task entered the active state.");

    registerOutputField("end_timestamp", SAKURA_STRING_TYPE)
        .setComment("Timestamp in UTC when the task was finished.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
ShowTaskV1M0::runTask(BlossomIO& blossomIO,
                      const Hanami::UserContext& userContext,
                      BlossomStatus& status,
                      Hanami::ErrorContainer& error)
{
    const std::string clusterUuid = blossomIO.input["cluster_uuid"];
    const std::string taskUuid = blossomIO.input["uuid"];

    // check if user exist within the table
    json getResult;
    ReturnStatus ret = ClusterTable::getInstance()->getCluster(
        getResult, clusterUuid, userContext, false, error);
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Cluster with uuid '" + clusterUuid + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    // get cluster
    Cluster* cluster = ClusterHandler::getInstance()->getCluster(clusterUuid);
    if (cluster == nullptr) {
        status.errorMessage = "Cluster with UUID '" + clusterUuid + "'not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    const Task* task = cluster->getTask(taskUuid);
    TaskProgress progress = task->progress;

    // get basic information
    blossomIO.output["uuid"] = taskUuid;
    blossomIO.output["name"] = task->name;
    blossomIO.output["current_cycle"] = progress.currentCyle;
    blossomIO.output["total_number_of_cycles"] = progress.totalNumberOfCycles;
    blossomIO.output["queue_timestamp"] = Hanami::serializeTimePoint(progress.queuedTimeStamp);

    // get timestamps
    if (progress.state == QUEUED_TASK_STATE) {
        blossomIO.output["state"] = "queued";
        blossomIO.output["start_timestamp"] = "-";
        blossomIO.output["end_timestamp"] = "-";
    }
    else if (progress.state == ACTIVE_TASK_STATE) {
        blossomIO.output["state"] = "active";
        blossomIO.output["start_timestamp"]
            = Hanami::serializeTimePoint(progress.startActiveTimeStamp);
        blossomIO.output["end_timestamp"] = "-";
    }
    else if (progress.state == ABORTED_TASK_STATE) {
        blossomIO.output["state"] = "aborted";
        blossomIO.output["start_timestamp"]
            = Hanami::serializeTimePoint(progress.startActiveTimeStamp);
        blossomIO.output["end_timestamp"] = "-";
    }
    else if (progress.state == FINISHED_TASK_STATE) {
        blossomIO.output["state"] = "finished";
        blossomIO.output["start_timestamp"]
            = Hanami::serializeTimePoint(progress.startActiveTimeStamp);
        blossomIO.output["end_timestamp"] = Hanami::serializeTimePoint(progress.endActiveTimeStamp);
    }

    return true;
}
