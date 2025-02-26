/**
 * @file        list_task_v1_0.cpp
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

#include "list_task_v1_0.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_handler.h>
#include <database/cluster_table.h>
#include <hanami_root.h>

ListTaskV1M0::ListTaskV1M0() : Blossom("List all visible tasks of a specific cluster.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("cluster_uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the cluster, whos tasks should be listed")
        .setRegex(UUID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    json headerMatch = json::array();
    headerMatch.push_back("uuid");
    headerMatch.push_back("state");
    headerMatch.push_back("percentage");
    headerMatch.push_back("queued");
    headerMatch.push_back("start");
    headerMatch.push_back("end");

    registerOutputField("header", SAKURA_ARRAY_TYPE)
        .setComment("Array with the namings all columns of the table.")
        .setMatch(headerMatch);

    registerOutputField("body", SAKURA_ARRAY_TYPE)
        .setComment("Array with all rows of the table, which array arrays too.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
ListTaskV1M0::runTask(BlossomIO& blossomIO,
                      const Hanami::UserContext& userContext,
                      BlossomStatus& status,
                      Hanami::ErrorContainer& error)
{
    const std::string clusterUuid = blossomIO.input["cluster_uuid"];

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

    // get progress of all tasks
    std::map<std::string, TaskProgress> progressOverview;
    cluster->getAllProgress(progressOverview);

    // init table-header
    Hanami::TableItem result;
    result.addColumn("uuid");
    result.addColumn("state");
    result.addColumn("percentage");
    result.addColumn("queued");
    result.addColumn("start");
    result.addColumn("end");

    // build table-content
    for (const auto& [id, progress] : progressOverview) {
        const float percentage
            = (1.0f / (float)progress.totalNumberOfCycles) * (float)progress.currentCyle;
        if (progress.state == QUEUED_TASK_STATE) {
            result.addRow(
                std::vector<std::string>{id,
                                         "queued",
                                         std::to_string(percentage),
                                         Hanami::serializeTimePoint(progress.queuedTimeStamp),
                                         "-",
                                         "-"});
        }
        else if (progress.state == ACTIVE_TASK_STATE) {
            result.addRow(
                std::vector<std::string>{id,
                                         "active",
                                         std::to_string(percentage),
                                         Hanami::serializeTimePoint(progress.queuedTimeStamp),
                                         Hanami::serializeTimePoint(progress.startActiveTimeStamp),
                                         "-"});
        }
        else if (progress.state == ABORTED_TASK_STATE) {
            result.addRow(
                std::vector<std::string>{id,
                                         "aborted",
                                         std::to_string(percentage),
                                         Hanami::serializeTimePoint(progress.queuedTimeStamp),
                                         Hanami::serializeTimePoint(progress.startActiveTimeStamp),
                                         "-"});
        }
        else if (progress.state == FINISHED_TASK_STATE) {
            result.addRow(
                std::vector<std::string>{id,
                                         "finished",
                                         std::to_string(percentage),
                                         Hanami::serializeTimePoint(progress.queuedTimeStamp),
                                         Hanami::serializeTimePoint(progress.startActiveTimeStamp),
                                         Hanami::serializeTimePoint(progress.endActiveTimeStamp)});
        }
    }

    // create output
    blossomIO.output["header"] = result.getInnerHeader();
    blossomIO.output["body"] = result.getBody();

    return true;
}
