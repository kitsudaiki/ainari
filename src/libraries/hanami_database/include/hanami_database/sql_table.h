/**
 * @file       sql_table.h
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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

#ifndef HANAMI_DATABASE_SQL_TABLE_H
#define HANAMI_DATABASE_SQL_TABLE_H

#include <hanami_common/enums.h>
#include <hanami_common/logger.h>
#include <hanami_common/uuid.h>

#include <nlohmann/json.hpp>
#include <string>
#include <vector>

using json = nlohmann::json;

namespace Hanami
{
class SqlDatabase;

class SqlTable
{
   public:
    SqlTable(SqlDatabase* db);
    virtual ~SqlTable();

    bool initTable(ErrorContainer& error);
    void createDocumentation(std::string& docu);

    uint64_t getNumberOfColumns() const;

   protected:
    enum DbVataValueTypes {
        STRING_TYPE = 0,
        INT_TYPE = 1,
        BOOL_TYPE = 2,
        FLOAT_TYPE = 3,
        BASE64_TYPE = 4
    };

    struct DbHeaderEntry {
        std::string name = "";
        int maxLength = -1;
        DbVataValueTypes type = STRING_TYPE;
        bool isPrimary = false;
        bool allowNull = false;
        bool hide = false;

        DbHeaderEntry& setMaxLength(const int maxLength)
        {
            this->maxLength = maxLength;
            return *this;
        }
        DbHeaderEntry& setAllowNull()
        {
            this->allowNull = true;
            return *this;
        }
        DbHeaderEntry& setIsPrimary()
        {
            this->isPrimary = true;
            return *this;
        }
        DbHeaderEntry& hideValue()
        {
            this->hide = true;
            return *this;
        }
    };

    struct RequestCondition {
        std::string colName;
        std::string value;

        RequestCondition(const std::string& colName, const std::string& value)
        {
            this->colName = colName;
            this->value = value;
        }
    };

    DbHeaderEntry& registerColumn(const std::string& name, const DbVataValueTypes type);

    bool insertToDb(json& values, const std::string& creator, ErrorContainer& error);
    ReturnStatus updateInDb(std::vector<RequestCondition>& conditions,
                            json& updates,
                            const std::string& updater,
                            ErrorContainer& error);
    bool getAllFromDb(TableItem& resultTable,
                      ErrorContainer& error,
                      const bool showHiddenValues,
                      const uint64_t positionOffset = 0,
                      const uint64_t numberOfRows = 0);
    ReturnStatus getFromDb(TableItem& resultTable,
                           std::vector<RequestCondition>& conditions,
                           const bool showHiddenValues,
                           const bool expectAtLeastOne,
                           ErrorContainer& error,
                           const uint64_t positionOffset = 0,
                           const uint64_t numberOfRows = 0);
    ReturnStatus getFromDb(json& result,
                           std::vector<RequestCondition>& conditions,
                           const bool showHiddenValues,
                           const bool expectAtLeastOne,
                           ErrorContainer& error,
                           const uint64_t positionOffset = 0,
                           const uint64_t numberOfRows = 0);
    long getNumberOfRows(ErrorContainer& error);
    bool deleteAllFromDb(const std::string& deleter, ErrorContainer& error);
    ReturnStatus deleteFromDb(std::vector<RequestCondition>& conditions,
                              const std::string& deleter,
                              ErrorContainer& error);

   protected:
    std::string m_tableName = "";
    std::vector<DbHeaderEntry> m_tableHeader;
    bool m_hasBase64Column = false;

   private:
    SqlDatabase* m_db = nullptr;

    const std::string createTableCreateQuery();
    const std::string createSelectQuery(const std::vector<RequestCondition>& conditions,
                                        const bool showHiddenValues,
                                        const uint64_t positionOffset,
                                        const uint64_t numberOfRows);
    const std::string createUpdateQuery(const std::vector<RequestCondition>& conditions,
                                        const json& updates);
    const std::string createInsertQuery(const std::vector<std::string>& values);
    const std::string createCountQuery();

    void processGetResult(json& result, TableItem& tableContent, const bool showHiddenValues);
    bool processBase64Entries(TableItem& tableContent, const bool showHiddenValues);
};

}  // namespace Hanami

#endif  // HANAMI_DATABASE_SQL_TABLE_H
