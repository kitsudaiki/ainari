/**
 * @file        hanami_root.cpp
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

#include <api/endpoint_processing/http_server.h>
#include <api/endpoint_processing/http_websocket_thread.h>
#include <core/cluster/cluster.h>
#include <core/cluster/cluster_handler.h>
#include <core/cluster/cluster_init.h>
#include <core/processing/physical_host.h>
#include <core/temp_file_handler.h>
#include <core/thread_binder.h>
#include <database/audit_log_table.h>
#include <database/checkpoint_table.h>
#include <database/cluster_table.h>
#include <database/dataset_table.h>
#include <database/error_log_table.h>
#include <database/projects_table.h>
#include <database/tempfile_table.h>
#include <database/users_table.h>
#include <hanami_common/files/text_file.h>
#include <hanami_common/functions/file_functions.h>
#include <hanami_common/logger.h>
#include <hanami_config/config_handler.h>
#include <hanami_database/sql_database.h>
#include <hanami_hardware/host.h>
#include <hanami_hardware/power_measuring.h>
#include <hanami_hardware/speed_measuring.h>
#include <hanami_hardware/temperature_measuring.h>
#include <hanami_policies/policy.h>
#include <hanami_root.h>

// init static variables
Hanami::GpuInterface* HanamiRoot::gpuInterface = nullptr;
HanamiRoot* HanamiRoot::root = nullptr;
PhysicalHost* HanamiRoot::physicalHost = nullptr;
HttpServer* HanamiRoot::httpServer = nullptr;
std::string HanamiRoot::tokenKey{};

/**
 * @brief constructor
 */
HanamiRoot::HanamiRoot() { root = this; }

/**
 * @brief destructor
 */
HanamiRoot::~HanamiRoot() {}

/**
 * @brief init root-object
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HanamiRoot::init(Hanami::ErrorContainer& error)
{
    srand(time(NULL));

    if (initDataDirectory(error) == false) {
        error.addMessage("Failed to initialize directories");
        return false;
    }

    if (initDatabase(error) == false) {
        error.addMessage("Failed to initialize database");
        return false;
    }

    clearCluster(error);

    if (initJwt(error) == false) {
        error.addMessage("Failed to initialize jwt");
        return false;
    }

    if (initPolicies(error) == false) {
        error.addMessage("Failed to initialize policies");
        return false;
    }

    if (initHttpServer() == false) {
        error.addMessage("initializing http-server failed");
        LOG_ERROR(error);
        return false;
    }

    // inti hosts
    physicalHost = new PhysicalHost();
    if (physicalHost->init(error) == false) {
        LOG_ERROR(error);
        return false;
    }

    // create thread-binder
    if (ThreadBinder::getInstance()->init(error) == false) {
        error.addMessage("failed to init thread-binder");
        LOG_ERROR(error);
        return false;
    }
    ThreadBinder::getInstance()->startThread();

    // start monitoring
    // TODO: re-enable measuring-threads
    // PowerMeasuring::getInstance()->startThread();
    // SpeedMeasuring::getInstance()->startThread();
    // TemperatureMeasuring::getInstance()->startThread();

    return true;
}

/**
 * @brief init database
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HanamiRoot::initDatabase(Hanami::ErrorContainer& error)
{
    bool success = false;

    // read database-path from config
    Hanami::SqlDatabase* database = Hanami::SqlDatabase::getInstance();
    const std::string databasePath = GET_STRING_CONFIG("DEFAULT", "database", success);
    LOG_DEBUG("database-path: '" + databasePath + "'");
    if (success == false) {
        error.addMessage("No database-path defined in config.");
        LOG_ERROR(error);
        return false;
    }

    // initalize database
    if (database->initDatabase(databasePath, error) == false) {
        error.addMessage("Failed to initialize sql-database.");
        LOG_ERROR(error);
        return false;
    }

    // initialize cluster-table
    ClusterTable* clustersTable = ClusterTable::getInstance();
    if (clustersTable->initTable(error) == false) {
        error.addMessage("Failed to initialize cluster-table in database.");
        LOG_ERROR(error);
        return false;
    }

    // initialize projects-table
    ProjectTable* projectsTable = ProjectTable::getInstance();
    if (projectsTable->initTable(error) == false) {
        error.addMessage("Failed to initialize project-table in database.");
        return false;
    }

    // initialize users-table
    UserTable* usersTable = UserTable::getInstance();
    if (usersTable->initTable(error) == false) {
        error.addMessage("Failed to initialize user-table in database.");
        return false;
    }
    if (usersTable->initNewAdminUser(error) == false) {
        error.addMessage("Failed to initialize new admin-user even this is necessary.");
        return false;
    }

    // initialize dataset-table
    DataSetTable* datasetTable = DataSetTable::getInstance();
    if (datasetTable->initTable(error) == false) {
        error.addMessage("Failed to initialize dataset-table in database.");
        LOG_ERROR(error);
        return false;
    }

    // initialize checkpoint-table
    CheckpointTable* clusterCheckpointTable = CheckpointTable::getInstance();
    if (clusterCheckpointTable->initTable(error) == false) {
        error.addMessage("Failed to initialize checkpoint-table in database.");
        LOG_ERROR(error);
        return false;
    }

    // initialize tempfile-table
    TempfileTable* tempfileTable = TempfileTable::getInstance();
    if (tempfileTable->initTable(error) == false) {
        error.addMessage("Failed to initialize tempfile-table in database.");
        LOG_ERROR(error);
        return false;
    }

    // initialize error-log-table
    ErrorLogTable* errorLogTable = ErrorLogTable::getInstance();
    if (errorLogTable->initTable(error) == false) {
        error.addMessage("Failed to initialize error-log-table in database.");
        LOG_ERROR(error);
        return false;
    }

    // initialize audit-log-table
    AuditLogTable* auditLogTable = AuditLogTable::getInstance();
    if (auditLogTable->initTable(error) == false) {
        error.addMessage("Failed to initialize audit-log-table in database.");
        LOG_ERROR(error);
        return false;
    }

    return true;
}

/**
 * @brief initialze http server
 *
 * @return true, if successful, else false
 */
bool
HanamiRoot::initHttpServer()
{
    bool success = false;

    // check if http is enabled
    if (GET_BOOL_CONFIG("http", "enable", success) == false) {
        return true;
    }

    // get stuff from config
    const uint16_t port = GET_INT_CONFIG("http", "port", success);
    const std::string ip = GET_STRING_CONFIG("http", "ip", success);
    const uint32_t numberOfThreads = GET_INT_CONFIG("http", "number_of_threads", success);

    // create server
    httpServer = new HttpServer(ip, port);
    httpServer->startThread();

    // start threads
    for (uint32_t i = 0; i < numberOfThreads; i++) {
        const std::string name = "HttpWebsocketThread";
        HttpWebsocketThread* httpWebsocketThread = new HttpWebsocketThread(name);
        httpWebsocketThread->startThread();
        m_threads.push_back(httpWebsocketThread);
    }

    return true;
}

/**
 * @brief HanamiRoot::initDataDirectory
 * @return
 */
bool
HanamiRoot::initDataDirectory(Hanami::ErrorContainer& error)
{
    bool success = false;

    const std::string datasetsPath = GET_STRING_CONFIG("storage", "dataset_location", success);
    if (success == false) {
        error.addMessage("No dataset_location defined in config.");
        return false;
    }

    if (createDirectory(datasetsPath, error) == false) {
        error.addMessage("Failed to create directory for datasets.");
        return false;
    }

    const std::string checkpointsPath
        = GET_STRING_CONFIG("storage", "checkpoint_location", success);
    if (success == false) {
        error.addMessage("No checkpoint_location defined in config.");
        return false;
    }

    if (createDirectory(checkpointsPath, error) == false) {
        error.addMessage("Failed to create directory for checkpoints.");
        return false;
    }

    const std::string tempfilesPath = GET_STRING_CONFIG("storage", "tempfile_location", success);
    if (success == false) {
        error.addMessage("No tempfile_location defined in config.");
        return false;
    }

    if (createDirectory(tempfilesPath, error) == false) {
        error.addMessage("Failed to create directory for tempfiles.");
        return false;
    }

    return true;
}

/**
 * @brief init policies
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HanamiRoot::initPolicies(Hanami::ErrorContainer& error)
{
    bool success = false;

    // read policy-file-path from config
    const std::string policyFilePath = GET_STRING_CONFIG("auth", "policies", success);
    if (success == false) {
        error.addMessage("No policy-file defined in config.");
        return false;
    }

    // read policy-file
    std::string policyFileContent;
    if (Hanami::readFile(policyFileContent, policyFilePath, error) == false) {
        error.addMessage("Failed to read policy-file");
        return false;
    }

    // parse policy-file
    Hanami::Policy* policies = Hanami::Policy::getInstance();
    if (policies->parse(policyFileContent, error) == false) {
        error.addMessage("Failed to parser policy-file");
        return false;
    }

    return true;
}

/**
 * @brief init jwt-class to validate incoming requested
 *
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HanamiRoot::initJwt(Hanami::ErrorContainer& error)
{
    bool success = false;

    // read jwt-token-key from config
    const std::string tokenKeyPath = GET_STRING_CONFIG("auth", "token_key_path", success);
    if (success == false) {
        error.addMessage("No token_key_path defined in config.");
        return false;
    }

    std::string tokenKeyString;
    if (Hanami::readFile(tokenKeyString, tokenKeyPath, error) == false) {
        error.addMessage("Failed to read token-file '" + tokenKeyPath + "'");
        return false;
    }

    // init jwt for token create and sign
    tokenKey = tokenKeyString;

    return true;
}

/**
 * @brief delete all clusters, because after a restart these are broken
 */
void
HanamiRoot::clearCluster(Hanami::ErrorContainer& error)
{
    Hanami::UserContext context;
    context.userId = "HANAMI_RESTART";
    ClusterTable::getInstance()->deleteAllCluster(context, error);
    // TODO: if a checkpoint exist for a broken cluster, then the cluster should be
    //       restored with the last available checkpoint
}
