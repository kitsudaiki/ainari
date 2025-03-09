/**
 * @file        download_dataset_content_v1_0.cpp
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

#include "download_dataset_content_v1_0.h"

#include <core/io/data_set/dataset_file_io.h>
#include <database/dataset_table.h>
#include <hanami_config/config_handler.h>
#include <hanami_root.h>

DownloadDatasetContentV1M0::DownloadDatasetContentV1M0()
    : Blossom("Compare a list of values with a dataset to check accuracy.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the dataset of which the content was requested.")
        .setRegex(UUID_REGEX);

    registerInputField("column_name", SAKURA_STRING_TYPE)
        .setComment("Name of the column to read from the dataset.")
        .setRegex(NAME_REGEX);

    registerInputField("row_offset", SAKURA_INT_TYPE)
        .setComment("The row number where to start read of the data.")
        .setLimit(0, 1000000000)
        .setDefault(0);

    registerInputField("number_of_rows", SAKURA_INT_TYPE)
        .setComment("Number of rows to read from the dataset starting by the offset.")
        .setLimit(1, 10000)
        .setDefault(1);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("data", SAKURA_ARRAY_TYPE).setComment("Read content for the dataset.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}
/**
 * @brief runTask
 */
bool
DownloadDatasetContentV1M0::runTask(BlossomIO& blossomIO,
                                    const Hanami::UserContext& userContext,
                                    BlossomStatus& status,
                                    Hanami::ErrorContainer& error)
{
    const std::string datasetUuid = blossomIO.input["uuid"];
    const std::string columnName = blossomIO.input["column_name"];
    const uint64_t rowOffset = blossomIO.input["row_offset"];
    const uint64_t numberOfRows = blossomIO.input["number_of_rows"];

    DataSetFileHandle datasetFileHandle;

    // open files
    ReturnStatus ret = getFileHandle(datasetFileHandle, datasetUuid, userContext, status, error);
    if (ret != OK) {
        return false;
    }

    // check if requested column even exist in dataset
    if (datasetFileHandle.description.contains(columnName) == false) {
        status.errorMessage = "Column with name '" + columnName
                              + "' was not found in dataset with UUID '" + datasetUuid + "'";
        status.statusCode = NOT_FOUND_RTYPE;
        return false;
    }

    // set file-selectors
    const json range = datasetFileHandle.description[columnName];
    datasetFileHandle.readSelector.columnStart = range["column_start"];
    datasetFileHandle.readSelector.columnEnd = range["column_end"];

    // check requested offset
    if (rowOffset >= datasetFileHandle.header.numberOfRows) {
        status.statusCode = INVALID_INPUT;
        status.errorMessage
            = "Requested row_offset is too big for the dataset with UUID '" + datasetUuid + "'";
        error.addMessage(status.errorMessage);
        return false;
    }
    datasetFileHandle.readSelector.startRow = rowOffset;

    // check and set number of requested rows
    if (rowOffset + numberOfRows > datasetFileHandle.header.numberOfRows) {
        datasetFileHandle.readSelector.endRow = datasetFileHandle.header.numberOfRows - rowOffset;
    }
    else {
        datasetFileHandle.readSelector.endRow = rowOffset + numberOfRows;
    }

    // init buffer for output
    const uint64_t numberOfColumns
        = datasetFileHandle.readSelector.columnEnd - datasetFileHandle.readSelector.columnStart;
    std::vector<float> datasetOutput(numberOfColumns, 0.0f);
    blossomIO.output["data"] = json::array();

    // check files
    for (uint64_t rowNumber = datasetFileHandle.readSelector.startRow;
         rowNumber < datasetFileHandle.readSelector.endRow;
         rowNumber++)
    {
        if (getDataFromDataSet(datasetOutput, datasetFileHandle, rowNumber, error) != OK) {
            status.statusCode = INVALID_INPUT;
            status.errorMessage
                = "Dataset with UUID '" + datasetUuid + "' is invalid and can not be read.";
            error.addMessage(status.errorMessage);
            return false;
        }

        // convert values into output
        json rowData = json::array();
        for (const float val : datasetOutput) {
            rowData.push_back(val);
        }
        blossomIO.output["data"].push_back(rowData);
    }

    return true;
}

/**
 * @brief CheckMnistDataSet::getFileHandle
 * @param fileHandle
 * @param uuid
 * @param context
 * @param status
 * @param error
 * @return
 */
ReturnStatus
DownloadDatasetContentV1M0::getFileHandle(DataSetFileHandle& fileHandle,
                                          const std::string uuid,
                                          const Hanami::UserContext userContext,
                                          BlossomStatus& status,
                                          Hanami::ErrorContainer& error)
{
    json dbOutput;
    ReturnStatus ret
        = DataSetTable::getInstance()->getDataSet(dbOutput, uuid, userContext, true, error);
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return ret;
    }
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Dataset with uuid '" + uuid + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return ret;
    }

    // get file information
    const std::string location = dbOutput["location"];
    if (openDataSetFile(fileHandle, location, error) != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return ERROR;
    }

    return OK;
}
