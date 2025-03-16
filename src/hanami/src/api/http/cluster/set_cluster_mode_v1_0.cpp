/**
 * @file        set_cluster_mode_v1_0.cpp
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

#include "set_cluster_mode_v1_0.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_handler.h>
#include <database/cluster_table.h>
#include <hanami_root.h>

SetClusterModeV1M0::SetClusterModeV1M0() : Blossom("Set mode of the cluster.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);
    errorCodes.push_back(CONFLICT_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the cluster.")
        .setRegex(UUID_REGEX);

    registerInputField("connection_uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the connection for input and output.")
        .setRegex(UUID_REGEX)
        .setRequired(false);

    registerInputField("new_state", SAKURA_STRING_TYPE)
        .setComment("New desired state for the cluster.")
        .setRegex("^(TASK|DIRECT)$");

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the cluster.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the cluster.");

    registerOutputField("new_state", SAKURA_STRING_TYPE)
        .setComment("New desired state for the cluster.");

    registerOutputField("created_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when resource was created.");

    registerOutputField("created_by", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the resource.");

    registerOutputField("updated_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when resource was updated.");

    registerOutputField("updated_by", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who updated the resource.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
SetClusterModeV1M0::runTask(BlossomIO& blossomIO,
                            const Hanami::UserContext& userContext,
                            BlossomStatus& status,
                            Hanami::ErrorContainer& error)
{
    const std::string clusterUuid = blossomIO.input["uuid"];
    // const std::string connectionUuid = blossomIO.input["connection_uuid"];
    const std::string newState = blossomIO.input["new_state"];

    // get data from table
    json clusterResult;
    ReturnStatus ret = ClusterTable::getInstance()->getCluster(
        blossomIO.output, clusterUuid, userContext, false, error);
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
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        error.addMessage("Cluster with UUID '" + clusterUuid
                         + "'not found even it exists within the database");
        return false;
    }

    // switch mode of cluster
    if (cluster->setClusterState(newState) == false) {
        status.errorMessage = "Can not switch Cluster with uuid '" + clusterUuid + "' to new mode '"
                              + newState + "'";
        // TODO: get exact reason, why it was not successful
        status.statusCode = CONFLICT_RTYPE;
        status.errorMessage = "Can not switch cluster to '" + newState + "'";
        error.addMessage(status.errorMessage);
        return false;
    }

    blossomIO.output["new_state"] = newState;

    // remove irrelevant fields
    blossomIO.output.erase("owner_id");
    blossomIO.output.erase("project_id");
    blossomIO.output.erase("visibility");

    return true;
}
