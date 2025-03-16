/**
 * @file        cluster_table.cpp
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

#include "cluster_table.h"

#include <hanami_common/functions/string_functions.h>
#include <hanami_common/items/table_item.h>
#include <hanami_database/sql_database.h>

ClusterTable* ClusterTable::instance = nullptr;

/**
 * @brief constructor
 */
ClusterTable::ClusterTable() : HanamiSqlTable(Hanami::SqlDatabase::getInstance())
{
    m_tableName = "clusters";
}

/**
 * @brief destructor
 */
ClusterTable::~ClusterTable() {}

/**
 * @brief add a new cluster to the database
 *
 * @param clusterData cluster-entry to add to database
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if conflict, ERROR in case of internal error
 */
ReturnStatus
ClusterTable::addCluster(const ClusterDbEntry& clusterData,
                         const Hanami::UserContext& userContext,
                         Hanami::ErrorContainer& error)
{
    json clusterDataJson;

    clusterDataJson["name"] = clusterData.name;
    clusterDataJson["uuid"] = clusterData.uuid;
    clusterDataJson["visibility"] = clusterData.visibility;

    const ReturnStatus ret = addWithContext(clusterDataJson, userContext, error);
    if (ret != OK) {
        error.addMessage("Failed to add cluster-meta to database");
        return ret;
    }

    return OK;
}

/**
 * @brief get a cluster from the database by his name
 *
 * @param result reference for the result-output in case that a cluster with this name was found
 * @param clusterUuid uuid of the requested cluster
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
ClusterTable::getCluster(ClusterDbEntry& result,
                         const std::string& clusterUuid,
                         const Hanami::UserContext& userContext,
                         Hanami::ErrorContainer& error)
{
    json jsonRet;
    const ReturnStatus ret = getCluster(jsonRet, clusterUuid, userContext, true, error);
    if (ret != OK) {
        return ret;
    }

    result.name = jsonRet["name"];
    result.ownerId = jsonRet["owner_id"];
    result.projectId = jsonRet["project_id"];
    result.uuid = jsonRet["uuid"];
    result.visibility = jsonRet["visibility"];
    result.createdAt = jsonRet["created_at"];
    result.createdBy = jsonRet["created_by"];
    result.updatedAt = jsonRet["updated_at"];
    result.updatedBy = jsonRet["updated_by"];

    return OK;
}

/**
 * @brief get a cluster from the database by his name
 *
 * @param result reference for the result-output in case that a cluster with this name was found
 * @param clusterUuid uuid of the requested cluster
 * @param userContext context-object with all user specific information
 * @param showHiddenValues set to true to also show as hidden marked fields
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
ClusterTable::getCluster(json& result,
                         const std::string& clusterUuid,
                         const Hanami::UserContext& userContext,
                         const bool showHiddenValues,
                         Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", clusterUuid);

    // get user from db
    const ReturnStatus ret
        = getWithContext(result, userContext, conditions, showHiddenValues, error);
    if (ret != OK) {
        error.addMessage("Failed to get cluster-meta with UUID '" + clusterUuid
                         + "' from database");
        return ret;
    }

    return OK;
}

/**
 * @brief get all clusters from the database table
 *
 * @param result reference for the result-output
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
ClusterTable::getAllCluster(Hanami::TableItem& result,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    if (getAllWithContext(result, userContext, conditions, error, false) != OK) {
        error.addMessage("Failed to get all cluster-meta from database");
        return false;
    }

    return true;
}

/**
 * @brief delete a cluster from the table
 *
 * @param clusterUuid uuid of the cluster to delete
 * @param userContext context-object with all user specific information
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
ClusterTable::deleteCluster(const std::string& clusterUuid,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error)
{
    std::vector<RequestCondition> conditions;
    conditions.emplace_back("uuid", clusterUuid);

    const ReturnStatus ret = deleteFromDbWithContext(conditions, userContext, error);
    if (ret != OK) {
        error.addMessage("Failed to delete cluster-meta with UUID '" + clusterUuid
                         + "' from database");
        return ret;
    }

    return OK;
}

/**
 * @brief delete all cluster from database. This currently used to avoid
 *        broken empty clusters after a restart of the backend
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
ClusterTable::deleteAllCluster(const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error)
{
    return deleteAllFromDb(userContext.userId, error);
}
