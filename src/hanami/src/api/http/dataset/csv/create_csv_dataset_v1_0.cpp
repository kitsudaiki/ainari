/**
 * @file        create_csv_dataset_v1_0.cpp
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

#include "create_csv_dataset_v1_0.h"

#include <core/temp_file_handler.h>
#include <database/dataset_table.h>
#include <hanami_common/files/binary_file.h>
#include <hanami_common/uuid.h>
#include <hanami_config/config_handler.h>
#include <hanami_crypto/common.h>
#include <hanami_root.h>

CreateCsvDataSetV1M0::CreateCsvDataSetV1M0() : Blossom("Init new csv-file dataset.")
{
    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("name", SAKURA_STRING_TYPE)
        .setComment("Name of the new dataset.")
        .setLimit(4, 254)
        .setRegex(NAME_REGEX);

    registerInputField("input_data_size", SAKURA_INT_TYPE)
        .setComment("Total size of the input-data.")
        .setLimit(1, 10000000000);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("created_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when dataset was created.");

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the new dataset.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the new dataset.");

    registerOutputField("owner_id", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the dataset.");

    registerOutputField("project_id", SAKURA_STRING_TYPE)
        .setComment("ID of the project, where the dataset belongs to.");

    registerOutputField("visibility", SAKURA_STRING_TYPE)
        .setComment("Visibility of the dataset (private, shared, public).");

    registerOutputField("task_uuid", SAKURA_STRING_TYPE)
        .setComment(
            "In case that this dataset was created by a request-task, this contains the UUID of "
            "the task for identifaction.");

    registerOutputField("uuid_input_file", SAKURA_STRING_TYPE)
        .setComment("UUID to identify the file for date upload of input-data.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

bool
CreateCsvDataSetV1M0::runTask(BlossomIO& blossomIO,
                              const Hanami::UserContext& userContext,
                              BlossomStatus& status,
                              Hanami::ErrorContainer& error)
{
    const std::string name = blossomIO.input["name"];
    const long inputDataSize = blossomIO.input["input_data_size"];
    const std::string uuid = generateUuid().toString();

    // get directory to store data from config
    bool success = false;
    std::string targetFilePath = GET_STRING_CONFIG("storage", "dataset_location", success);
    if (success == false) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        error.addMessage("file-location to store dataset is missing in the config");
        return false;
    }

    // init temp-file for input-data
    std::string inputUuid;
    ReturnStatus ret = TempFileHandler::getInstance()->initNewFile(
        inputUuid, "input-file", uuid, inputDataSize, userContext, error);
    if (ret != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        error.addMessage("Failed to initialize temporary file for new input-data.");
        return false;
    }

    // build absolut file-path to store the file
    if (targetFilePath.at(targetFilePath.size() - 1) != '/') {
        targetFilePath.append("/");
    }
    targetFilePath.append(name + "_csv_" + userContext.userId);

    // register in database
    DataSetTable::DataSetDbEntry dbEntry;
    dbEntry.name = name;
    dbEntry.ownerId = userContext.userId;
    dbEntry.projectId = userContext.projectId;
    dbEntry.uuid = uuid;
    dbEntry.visibility = "private";
    dbEntry.location = targetFilePath;

    // add to database
    if (DataSetTable::getInstance()->addDataSet(dbEntry, userContext, error) != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    if (DataSetTable::getInstance()->getDataSet(blossomIO.output, uuid, userContext, true, error)
        != OK)
    {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // add values to output
    blossomIO.output["uuid_input_file"] = inputUuid;

    // remove blocked values from output
    blossomIO.output.erase("location");

    return true;
}
