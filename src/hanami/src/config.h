/**
 * @file        config.h
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

#ifndef HANAMI_CONFIG_H
#define HANAMI_CONFIG_H

#include <hanami_config/config_handler.h>

/**
 * @brief define all available entries in the config file with default-values
 */
inline void
registerConfigs()
{
    // DEFAULT-section
    const std::string defaultGroup = "DEFAULT";

    REGISTER_BOOL_CONFIG(defaultGroup, "debug")
        .setComment("Flag to enable debug-output in logging.")
        .setDefault(false);

    REGISTER_STRING_CONFIG(defaultGroup, "log_path")
        .setComment("Path to the directory, where the log-files should be written into.")
        .setDefault("/var/log");

    REGISTER_STRING_CONFIG(defaultGroup, "database")
        .setComment("Path to the sqlite3 database-file for all local sql-tables of hanami.")
        .setDefault("/etc/openhanami/hanami_db");

    // storage-section
    const std::string storageGroup = "storage";

    REGISTER_STRING_CONFIG(storageGroup, "dataset_location")
        .setComment("Local storage location, where all uploaded dataset should be written into.")
        .setDefault("/etc/openhanami/datasets");

    REGISTER_STRING_CONFIG(storageGroup, "checkpoint_location")
        .setComment("Local storage location, where all uploaded dataset should be written into.")
        .setDefault("/etc/openhanami/checkpoints");

    REGISTER_STRING_CONFIG(storageGroup, "tempfile_location")
        .setComment(
            "Local storage location, where all temporary files for upload and download should be "
            "cached.")
        .setDefault("/etc/openhanami/tempfiles");

    REGISTER_INT_CONFIG(storageGroup, "tempfile_timeout")
        .setComment("Number of minutes, until an inactive timefile is removed.")
        .setDefault(10);

    // processing-section
    const std::string processingGroup = "processing";

    REGISTER_FLOAT_CONFIG(processingGroup, "use_of_free_memory")
        .setComment(
            "Define the percentage allcating free memory on the host (example: 0.5 = 50%). "
            "Maximum is 90% and will be hard cut, in case a value higher than 0.9 is provided. "
            "The minimum is 0.1")
        .setDefault(0.5);

    // auth-section
    const std::string authGroup = "auth";

    REGISTER_STRING_CONFIG(authGroup, "token_key_path")
        .setComment("Local path to the file with the key for signing and validating the jwt-token.")
        .setRequired();

    REGISTER_INT_CONFIG(authGroup, "token_expire_time")
        .setComment("Number of seconds, until a jwt-token expired.")
        .setDefault(3600);

    REGISTER_STRING_CONFIG(authGroup, "policies")
        .setComment("Local path to the file with the endpoint-policies.")
        .setDefault("/etc/openhanami/policies");

    // http-section
    const std::string httpGroup = "http";

    REGISTER_BOOL_CONFIG(httpGroup, "enable")
        .setComment("Flag to enable the http-endpoint.")
        .setDefault(false);

    REGISTER_STRING_CONFIG(httpGroup, "ip")
        .setComment("IP-address, where the http-server should listen.")
        .setDefault("0.0.0.0");

    REGISTER_INT_CONFIG(httpGroup, "port")
        .setComment("Port, where the http-server should listen.")
        .setDefault(11418);

    REGISTER_INT_CONFIG(httpGroup, "number_of_threads")
        .setComment("Number of threads in the thread-pool for processing http-requests.")
        .setDefault(4);
}

#endif  // HANAMI_CONFIG_H
