/**
 * @file        show_cluster_v1_0.cpp
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

#include "show_cluster_v1_0.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_handler.h>
#include <database/cluster_table.h>
#include <hanami_root.h>

ShowClusterV1M0::ShowClusterV1M0() : Blossom("Show information of a specific cluster.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("uuid of the cluster.")
        .setRegex(UUID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the cluster.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the cluster.");

    registerOutputField("owner_id", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the cluster.");

    registerOutputField("project_id", SAKURA_STRING_TYPE)
        .setComment("ID of the project, where the cluster belongs to.");

    registerOutputField("visibility", SAKURA_STRING_TYPE)
        .setComment("Visibility of the cluster (private, shared, public).");

    registerOutputField("number_of_blocks", SAKURA_INT_TYPE)
        .setComment("Number of blocks in the cluster.");

    registerOutputField("number_of_sections", SAKURA_INT_TYPE)
        .setComment("Number of synapse-sections in the cluster.");

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
ShowClusterV1M0::runTask(BlossomIO& blossomIO,
                         const Hanami::UserContext& userContext,
                         BlossomStatus& status,
                         Hanami::ErrorContainer& error)
{
    const std::string clusterUuid = blossomIO.input["uuid"];

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

    // add metrics to output
    blossomIO.output["number_of_blocks"] = cluster->metrics.numberOfBlocks;
    blossomIO.output["number_of_sections"] = cluster->metrics.numberOfSections;

    return true;
}
