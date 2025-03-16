/**
 * @file        test_table.h
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

#ifndef TESTTABLE_H
#define TESTTABLE_H

#include <hanami_database/sql_table.h>

namespace Hanami
{

class SqlDatabase;

class TestTable : public Hanami::SqlTable
{
   public:
    TestTable(Hanami::SqlDatabase* db);
    ~TestTable();

    bool addUser(json& data, ErrorContainer& error);
    ReturnStatus getUser(TableItem& resultTable,
                         const std::string& userID,
                         ErrorContainer& error,
                         const bool withHideValues = false);
    ReturnStatus getUser(json& resultItem,
                         const std::string& userID,
                         const bool showHiddenValues,
                         ErrorContainer& error);
    bool getAllUser(TableItem& resultItem,
                    ErrorContainer& error,
                    const bool showHiddenValues,
                    const uint64_t positionOffset = 0,
                    const uint64_t numberOfRows = 0);
    ReturnStatus deleteUser(const std::string& userID, ErrorContainer& error);
    ReturnStatus updateUser(const std::string& userID, json& values, ErrorContainer& error);
    long getNumberOfUsers(ErrorContainer& error);
};

}  // namespace Hanami

#endif  // TESTTABLE_H
