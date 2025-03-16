/**
 * @file        projects_table.h
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

#ifndef HANAMI_PROJECTS_TABLE_H
#define HANAMI_PROJECTS_TABLE_H

#include <database/generic_tables/hanami_sql_admin_table.h>
#include <hanami_common/logger.h>
#include <hanami_common/structs.h>

class ProjectTable : public HanamiSqlAdminTable
{
   public:
    static ProjectTable* getInstance()
    {
        if (instance == nullptr) {
            instance = new ProjectTable();
        }
        return instance;
    }

    struct ProjectDbEntry {
        std::string id = "";
        std::string name = "";
        std::string createdAt = "";
        std::string createdBy = "";
        std::string updatedAt = "";
        std::string updatedBy = "";
    };

    ~ProjectTable();

    ReturnStatus addProject(const ProjectDbEntry& userData,
                            const Hanami::UserContext& context,
                            Hanami::ErrorContainer& error);
    ReturnStatus getProject(ProjectDbEntry& result,
                            const std::string& projectName,
                            Hanami::ErrorContainer& error);
    ReturnStatus getProject(json& result,
                            const std::string& projectName,
                            const bool showHiddenValues,
                            Hanami::ErrorContainer& error);
    bool getAllProjects(Hanami::TableItem& result, Hanami::ErrorContainer& error);
    ReturnStatus deleteProject(const std::string& projectName,
                               const Hanami::UserContext& context,
                               Hanami::ErrorContainer& error);

   private:
    ProjectTable();
    static ProjectTable* instance;
};

#endif  // HANAMI_PROJECTS_TABLE_H
