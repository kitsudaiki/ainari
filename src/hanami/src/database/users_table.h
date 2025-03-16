/**
 * @file        users_database.h
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

#ifndef HANAMI_USERS_TABLE_H
#define HANAMI_USERS_TABLE_H

#include <database/generic_tables/hanami_sql_admin_table.h>
#include <hanami_common/logger.h>
#include <hanami_common/structs.h>

class UserTable : public HanamiSqlAdminTable
{
   public:
    static UserTable* getInstance()
    {
        if (instance == nullptr) {
            instance = new UserTable();
        }
        return instance;
    }

    struct UserProjectDbEntry {
        std::string projectId = "";
        std::string role = "";
        bool isProjectAdmin = false;
    };

    struct UserDbEntry {
        std::string id = "";
        std::string name = "";
        std::string salt = "";
        std::string pwHash = "";
        std::vector<UserProjectDbEntry> projects;
        bool isAdmin = false;
        std::string createdAt = "";
        std::string createdBy = "";
        std::string updatedAt = "";
        std::string updatedBy = "";
    };

    ~UserTable();

    bool initNewAdminUser(Hanami::ErrorContainer& error);

    ReturnStatus addUser(const UserDbEntry& userData,
                         const Hanami::UserContext& context,
                         Hanami::ErrorContainer& error);
    ReturnStatus getUser(UserDbEntry& result,
                         const std::string& userId,
                         Hanami::ErrorContainer& error);
    ReturnStatus getUser(json& result,
                         const std::string& userId,
                         const bool showHiddenValues,
                         Hanami::ErrorContainer& error);
    bool getAllUser(Hanami::TableItem& result, Hanami::ErrorContainer& error);
    ReturnStatus deleteUser(const std::string& userId,
                            const Hanami::UserContext& context,
                            Hanami::ErrorContainer& error);
    ReturnStatus updateProjectsOfUser(const std::string& userId,
                                      const std::vector<UserProjectDbEntry>& newProjects,
                                      const Hanami::UserContext& context,
                                      Hanami::ErrorContainer& error);

   private:
    UserTable();
    static UserTable* instance;

    bool getEnvVar(std::string& content, const std::string& key) const;

    ReturnStatus getAllAdminUser(Hanami::ErrorContainer& error);
};

#endif  // HANAMI_USERS_TABLE_H
