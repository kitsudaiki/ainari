/**
 * @file        get_dataset_v1_0.cpp
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

#include "get_dataset_v1_0.h"

#include <core/io/data_set/dataset_file_io.h>
#include <database/dataset_table.h>
#include <hanami_root.h>

GetDataSetV1M0::GetDataSetV1M0() : Blossom("Get information of a specific dataset.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the dataset set to delete.")
        .setRegex(UUID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the dataset.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the dataset.");

    registerOutputField("description", SAKURA_MAP_TYPE)
        .setComment("Description of the dataset content.");

    registerOutputField("task_uuid", SAKURA_STRING_TYPE)
        .setComment(
            "In case that this dataset was created by a request-task, this contains the UUID of "
            "the task for identifaction.");

    registerOutputField("owner_id", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the dataset.");

    registerOutputField("project_id", SAKURA_STRING_TYPE)
        .setComment("ID of the project, where the dataset belongs to.");

    registerOutputField("visibility", SAKURA_STRING_TYPE)
        .setComment("Visibility of the dataset (private, shared, public).");

    registerOutputField("version", SAKURA_STRING_TYPE).setComment("Version of the data-set file.");

    registerOutputField("number_of_rows", SAKURA_INT_TYPE).setComment("Number of rows.");

    registerOutputField("number_of_columns", SAKURA_INT_TYPE).setComment("Number of columns.");

    registerOutputField("created_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when dataset was created.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
GetDataSetV1M0::runTask(BlossomIO& blossomIO,
                        const Hanami::UserContext& userContext,
                        BlossomStatus& status,
                        Hanami::ErrorContainer& error)
{
    const std::string datasetUuid = blossomIO.input["uuid"];

    // get data-set information from database
    DataSetTable::DataSetDbEntry entry;
    const ReturnStatus ret
        = DataSetTable::getInstance()->getDataSet(entry, datasetUuid, userContext, error);
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Data-set with uuid '" + datasetUuid + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    // read header of file
    DataSetFileHandle fileHandle;
    if (openDataSetFile(fileHandle, entry.location, error) != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // create output
    blossomIO.output["uuid"] = datasetUuid;
    blossomIO.output["description"] = fileHandle.description;
    blossomIO.output["name"] = fileHandle.header.name.getName();
    blossomIO.output["task_uuid"] = entry.taskUuid;
    blossomIO.output["owner_id"] = entry.ownerId;
    blossomIO.output["project_id"] = entry.projectId;
    blossomIO.output["visibility"] = entry.visibility;
    blossomIO.output["version"] = std::string(fileHandle.header.version) + "."
                                  + std::string(fileHandle.header.minorVersion);
    blossomIO.output["number_of_rows"] = fileHandle.header.numberOfRows;
    blossomIO.output["number_of_columns"] = fileHandle.header.numberOfColumns;
    blossomIO.output["created_at"] = entry.createdAt;

    return true;
}
