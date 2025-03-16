/**
 * @file        dataset_table.h
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

#ifndef HANAMI_DATA_SET_TABLE_H
#define HANAMI_DATA_SET_TABLE_H

#include <database/generic_tables/hanami_sql_table.h>
#include <hanami_common/logger.h>

class DataSetTable : public HanamiSqlTable
{
   public:
    static DataSetTable* getInstance()
    {
        if (instance == nullptr) {
            instance = new DataSetTable();
        }
        return instance;
    }

    struct DataSetDbEntry {
        std::string uuid = "";
        std::string projectId = "";
        std::string ownerId = "";
        std::string visibility = "";
        std::string name = "";
        std::string taskUuid = "";
        std::string location = "";
        std::string createdAt = "";
        std::string createdBy = "";
        std::string updatedAt = "";
        std::string updatedBy = "";
    };

    ~DataSetTable();

    ReturnStatus addDataSet(const DataSetDbEntry& datasetData,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error);
    ReturnStatus getDataSet(DataSetDbEntry& result,
                            const std::string& datasetUuid,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error);
    ReturnStatus getDataSet(json& result,
                            const std::string& datasetUuid,
                            const Hanami::UserContext& userContext,
                            const bool showHiddenValues,
                            Hanami::ErrorContainer& error);
    bool getAllDataSet(Hanami::TableItem& result,
                       const Hanami::UserContext& userContext,
                       Hanami::ErrorContainer& error);
    ReturnStatus deleteDataSet(const std::string& uuid,
                               const Hanami::UserContext& userContext,
                               Hanami::ErrorContainer& error);

   private:
    DataSetTable();
    static DataSetTable* instance;
};

#endif  // HANAMI_DATA_SET_TABLE_H
