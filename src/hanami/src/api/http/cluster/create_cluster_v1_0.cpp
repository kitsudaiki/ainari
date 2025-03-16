/**
 * @file        create_cluster_v1_0.cpp
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

#include "create_cluster_v1_0.h"

#include <core/cluster/cluster.h>
#include <core/cluster/cluster_handler.h>
#include <core/processing/physical_host.h>
#include <database/cluster_table.h>
#include <hanami_common/buffer/data_buffer.h>
#include <hanami_crypto/common.h>
#include <hanami_root.h>

CreateClusterV1M0::CreateClusterV1M0() : Blossom("Create new cluster.")
{
    errorCodes.push_back(CONFLICT_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("name", SAKURA_STRING_TYPE)
        .setComment("Name for the new cluster.")
        .setLimit(4, 254)
        .setRegex(NAME_REGEX);

    registerInputField("template", SAKURA_STRING_TYPE)
        .setComment("Cluster-template as base64-string.")
        .setRequired(false);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the new created cluster.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the new created cluster.");

    registerOutputField("owner_id", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the new cluster.");

    registerOutputField("project_id", SAKURA_STRING_TYPE)
        .setComment("ID of the project, where the new cluster belongs to.");

    registerOutputField("visibility", SAKURA_STRING_TYPE)
        .setComment("Visibility of the new created cluster (private, shared, public).");

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
CreateClusterV1M0::runTask(BlossomIO& blossomIO,
                           const Hanami::UserContext& userContext,
                           BlossomStatus& status,
                           Hanami::ErrorContainer& error)
{
    const std::string clusterName = blossomIO.input["name"];
    const std::string base64Template = blossomIO.input["template"];

    // check if cluster-name already exist within the table
    ReturnStatus ret
        = ClusterTable::getInstance()->doesNameAlreadyExist(clusterName, userContext, error);
    if (ret == OK) {
        status.errorMessage = "Cluster with name '" + clusterName + "' already exist.";
        status.statusCode = CONFLICT_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    Hanami::ClusterMeta parsedCluster;
    if (base64Template != "") {
        // decode base64 formated template to check if valid base64-string
        Hanami::DataBuffer convertedTemplate;
        if (Hanami::decodeBase64(convertedTemplate, base64Template) == false) {
            status.errorMessage = "Uploaded template is not a valid base64-String.";
            status.statusCode = BAD_REQUEST_RTYPE;
            LOG_DEBUG(status.errorMessage);
            return false;
        }

        // parse cluster-template to validate syntax
        const std::string convertedTemplateStr(static_cast<const char*>(convertedTemplate.data),
                                               convertedTemplate.usedBufferSize);
        if (Hanami::parseCluster(&parsedCluster, convertedTemplateStr, error) == false) {
            status.errorMessage = "Uploaded template is not a valid cluster-template: \n";
            status.errorMessage += error.toString();
            status.statusCode = BAD_REQUEST_RTYPE;
            LOG_DEBUG(status.errorMessage);
            return false;
        }
    }

    ClusterTable::ClusterDbEntry dbEntry;
    dbEntry.uuid = generateUuid().toString();
    dbEntry.name = clusterName;
    dbEntry.projectId = userContext.projectId;
    dbEntry.ownerId = userContext.userId;
    dbEntry.visibility = "private";

    // add new user to table
    if (ClusterTable::getInstance()->addCluster(dbEntry, userContext, error) != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        error.addMessage("Failed to add cluster to database");
        return false;
    }

    // get new created user from database
    if (ClusterTable::getInstance()->getCluster(
            blossomIO.output, dbEntry.uuid, userContext, false, error)
        != OK)
    {
        error.addMessage("Failed to get cluster from database by name '" + clusterName + "'");
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    const std::string uuid = blossomIO.output["uuid"];

    // create new cluster
    Cluster* newCluster = new Cluster();
    if (newCluster->clusterHeader.name.setName(clusterName) == false) {
        error.addMessage("New cluster-name '" + clusterName
                         + "' too long, even this should be avoided by the API.");
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
    }
    if (base64Template != "") {
        // generate and initialize the cluster based on the cluster-templates
        if (newCluster->init(parsedCluster, uuid) == false) {
            error.addMessage("Failed to initialize cluster based on a template");
            status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
            delete newCluster;
            ClusterTable::getInstance()->deleteCluster(uuid, userContext, error);
            return false;
        }
    }

    // add to cluster-handler
    if (ClusterHandler::getInstance()->addCluster(uuid, newCluster) == false) {
        error.addMessage("Failed to add cluster to cluster-handler.");
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    return true;
}
