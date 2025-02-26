/**
 * @file        main.cpp
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

#include <api/websocket/cluster_io.h>
#include <args.h>
#include <config.h>
#include <database/audit_log_table.h>
#include <database/checkpoint_table.h>
#include <database/cluster_table.h>
#include <database/dataset_table.h>
#include <database/error_log_table.h>
#include <database/projects_table.h>
#include <database/users_table.h>
#include <documentation/generate_rest_api_docu.h>
#include <hanami_args/arg_parser.h>
#include <hanami_common/files/text_file.h>
#include <hanami_common/functions/time_functions.h>
#include <hanami_common/logger.h>
#include <hanami_config/config_handler.h>
#include <hanami_root.h>

#include <thread>

void
handleErrorCallback(const std::string& errorMessage,
                    const std::string& userId,
                    const std::string& values)
{
    Hanami::ErrorContainer error;
    ErrorLogTable::getInstance()->addErrorLogEntry(userId, "", "", values, errorMessage, error);
}

int
main(int argc, char* argv[])
{
    Hanami::ErrorContainer error;
    HanamiRoot rootObj;

    Hanami::initConsoleLogger(true);

    // create and init argument-parser
    Hanami::ArgParser argParser;
    registerArguments(&argParser);
    registerConfigs();

    // parse cli-input
    if (argParser.parse(argc, argv, error) == false) {
        LOG_ERROR(error);
        return 1;
    }

    // generate api-, config- and database-docu, if requested
    if (argParser.wasSet("generate_docu")) {
        namespace fs = std::filesystem;
        Hanami::ErrorContainer error;

        //-------------------------------------------------------------------------

        std::string openApiDocu = "";
        createOpenApiDocumentation(openApiDocu);
        fs::path complete = fs::current_path() / fs::path{"open_api_docu.json"};
        if (writeFile(complete.generic_string(), openApiDocu, error, true) == false) {
            LOG_ERROR(error);
            return 1;
        }
        std::cout << "Written OpenAPI-docu to " << complete << std::endl;

        //-------------------------------------------------------------------------

        std::string configDocu = "# Configs of Hanami\n\n";
        configDocu.append("!!! info\n\n");
        configDocu.append(
            "    This documeation was generated from the source-code to "
            "provide a maximum of consistency.\n\n");

        Hanami::ConfigHandler::getInstance()->createDocumentation(configDocu);
        complete = fs::current_path() / fs::path{"config.md"};
        if (writeFile(complete.generic_string(), configDocu, error, true) == false) {
            LOG_ERROR(error);
            return 1;
        }
        std::cout << "Written Config-docu to " << complete << std::endl;

        //-------------------------------------------------------------------------

        std::string dbDocu = "# Database-Tables\n\n";
        dbDocu.append("!!! info\n\n");
        dbDocu.append(
            "    This documeation was generated from the source-code to "
            "provide a maximum of consistency.\n\n");
        ClusterTable::getInstance()->createDocumentation(dbDocu);
        ProjectTable::getInstance()->createDocumentation(dbDocu);
        UserTable::getInstance()->createDocumentation(dbDocu);
        DataSetTable::getInstance()->createDocumentation(dbDocu);
        CheckpointTable::getInstance()->createDocumentation(dbDocu);
        ErrorLogTable::getInstance()->createDocumentation(dbDocu);
        AuditLogTable::getInstance()->createDocumentation(dbDocu);
        complete = fs::current_path() / fs::path{"db.md"};
        if (writeFile(complete.generic_string(), dbDocu, error, true) == false) {
            LOG_ERROR(error);
            return 1;
        }
        std::cout << "Written Database-docu to " << complete << std::endl;

        //-------------------------------------------------------------------------

        return 0;
    }

    // init and check config-file
    std::string configPath = argParser.getStringValue("config");
    if (configPath == "") {
        configPath = "/etc/openhanami/hanami.conf";
    }
    if (INIT_CONFIG(configPath, error) == false) {
        LOG_ERROR(error);
        return 1;
    }

    // get config-parameter for logger
    bool success = false;
    const bool enableDebug = GET_BOOL_CONFIG("DEFAULT", "debug", success);
    if (success == false) {
        return 1;
    }

    const std::string logPath = GET_STRING_CONFIG("DEFAULT", "log_path", success);
    if (success == false) {
        return 1;
    }

    // init logger
    Hanami::setErrorLogCallback(&handleErrorCallback);
    Hanami::initConsoleLogger(enableDebug);
    Hanami::initFileLogger(logPath, "hanami", enableDebug);

    // init root-object
    if (rootObj.init(error) == false) {
        LOG_ERROR(error);
        return 1;
    }

    // sleep forever
    std::this_thread::sleep_until(std::chrono::time_point<std::chrono::system_clock>::max());

    return 0;
}
