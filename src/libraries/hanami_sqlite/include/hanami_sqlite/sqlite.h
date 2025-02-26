/**
 * @file    sqlite.h
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

#ifndef SQLITE_H
#define SQLITE_H

#include <hanami_common/logger.h>
#include <sqlite3.h>

#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace Hanami
{
class TableItem;

class Sqlite
{
   public:
    Sqlite();
    ~Sqlite();

    bool initDB(const std::string& path, ErrorContainer& error);

    bool execSqlCommand(TableItem* resultTable, const std::string& command, ErrorContainer& error);

    bool closeDB();

   private:
    sqlite3* m_db = nullptr;
    int m_rc = 0;
};

}  // namespace Hanami

#endif  // SQLITE_H
