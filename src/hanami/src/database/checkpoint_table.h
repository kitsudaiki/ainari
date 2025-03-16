/**
 * @file        checkpoint_table.h
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

#ifndef HANAMI_CLUSTER_CHECKPOINT_TABLE_H
#define HANAMI_CLUSTER_CHECKPOINT_TABLE_H

#include <database/generic_tables/hanami_sql_table.h>
#include <hanami_common/logger.h>
#include <hanami_common/structs.h>

class CheckpointTable : public HanamiSqlTable
{
   public:
    static CheckpointTable* getInstance()
    {
        if (instance == nullptr) {
            instance = new CheckpointTable();
        }
        return instance;
    }

    struct CheckpointDbEntry {
        std::string uuid = "";
        std::string projectId = "";
        std::string ownerId = "";
        std::string visibility = "";
        std::string name = "";
        std::string location = "";
        std::string createdAt = "";
        std::string createdBy = "";
        std::string updatedAt = "";
        std::string updatedBy = "";
    };

    ~CheckpointTable();

    ReturnStatus addCheckpoint(const CheckpointDbEntry& checkpointData,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error);
    ReturnStatus getCheckpoint(CheckpointDbEntry& result,
                               const std::string& checkpointUuid,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error);
    ReturnStatus getCheckpoint(json& result,
                               const std::string& checkpointUuid,
                               const Hanami::UserContext& userContext,
                               const bool showHiddenValues,
                               Hanami::ErrorContainer& error);
    bool getAllCheckpoint(Hanami::TableItem& result,
                          const Hanami::UserContext& userContext,
                          const bool showHiddenValues,
                          Hanami::ErrorContainer& error);
    ReturnStatus deleteCheckpoint(const std::string& checkpointUuid,
                                  const Hanami::UserContext& userContext,
                                  Hanami::ErrorContainer& error);

   private:
    CheckpointTable();
    static CheckpointTable* instance;
};

#endif  // HANAMI_CLUSTER_CHECKPOINT_TABLE_H
