/**
 * @file        cluster_table.h
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

#ifndef HANAMI_CLUSTERTABLE_H
#define HANAMI_CLUSTERTABLE_H

#include <database/generic_tables/hanami_sql_table.h>
#include <hanami_common/logger.h>

class ClusterTable : public HanamiSqlTable
{
   public:
    static ClusterTable* getInstance()
    {
        if (instance == nullptr) {
            instance = new ClusterTable();
        }
        return instance;
    }

    struct ClusterDbEntry {
        std::string uuid = "";
        std::string projectId = "";
        std::string ownerId = "";
        std::string visibility = "";
        std::string name = "";
        std::string createdAt = "";
        std::string createdBy = "";
        std::string updatedAt = "";
        std::string updatedBy = "";
    };

    ~ClusterTable();

    ReturnStatus addCluster(const ClusterDbEntry& clusterData,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error);
    ReturnStatus getCluster(ClusterDbEntry& result,
                            const std::string& clusterUuid,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error);
    ReturnStatus getCluster(json& result,
                            const std::string& clusterUuid,
                            const Hanami::UserContext& userContext,
                            const bool showHiddenValues,
                            Hanami::ErrorContainer& error);
    bool getAllCluster(Hanami::TableItem& result,
                       const Hanami::UserContext& userContext,
                       Hanami::ErrorContainer& error);
    ReturnStatus deleteCluster(const std::string& clusterUuid,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error);
    bool deleteAllCluster(const Hanami::UserContext& userContext, Hanami::ErrorContainer& error);

   private:
    ClusterTable();
    static ClusterTable* instance;
};

#endif  // HANAMI_CLUSTERTABLE_H
