/**
 * @file        finalize_csv_dataset_v1_0.cpp
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

#include "finalize_csv_dataset_v1_0.h"

#include <core/io/data_set/dataset_file_io.h>
#include <core/temp_file_handler.h>
#include <database/dataset_table.h>
#include <hanami_common/files/binary_file.h>
#include <hanami_common/functions/file_functions.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_common/functions/vector_functions.h>
#include <hanami_crypto/common.h>
#include <hanami_root.h>

#include <regex>

FinalizeCsvDataSetV1M0::FinalizeCsvDataSetV1M0()
    : Blossom(
        "Finalize uploaded dataset by checking completeness of the "
        "uploaded and convert into generic format.")
{
    errorCodes.push_back(NOT_FOUND_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("uuid", SAKURA_STRING_TYPE)
        .setComment("UUID of the new dataset.")
        .setRegex(UUID_REGEX);

    registerInputField("uuid_input_file", SAKURA_STRING_TYPE)
        .setComment("UUID to identify the file for date upload of input-data.")
        .setRegex(UUID_REGEX);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("uuid", SAKURA_STRING_TYPE).setComment("UUID of the new dataset.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
FinalizeCsvDataSetV1M0::runTask(BlossomIO& blossomIO,
                                const Hanami::UserContext& userContext,
                                BlossomStatus& status,
                                Hanami::ErrorContainer& error)
{
    const std::string uuid = blossomIO.input["uuid"];
    const std::string inputUuid = blossomIO.input["uuid_input_file"];

    // get location from database
    DataSetTable::DataSetDbEntry result;
    const ReturnStatus ret
        = DataSetTable::getInstance()->getDataSet(result, uuid, userContext, error);
    if (ret == INVALID_INPUT) {
        status.errorMessage = "Data-set with uuid '" + uuid + "' not found";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // read input-data from temp-file
    Hanami::DataBuffer inputBuffer;
    if (TempFileHandler::getInstance()->getData(inputBuffer, inputUuid) == false) {
        status.errorMessage = "Input-data with uuid '" + inputUuid + "' not found.";
        status.statusCode = NOT_FOUND_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }

    // write data to file
    if (convertCsvData(result.location, result.name, inputBuffer, error) == false) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        error.addMessage("Failed to convert csv-data");
        return false;
    }

    // delete temp-files
    // TODO: error-handling
    TempFileHandler::getInstance()->removeData(inputUuid, userContext, error);

    // create output
    blossomIO.output["uuid"] = uuid;

    return true;
}

float
FinalizeCsvDataSetV1M0::convertField(std::string& cell)
{
    // int/long
    if (regex_match(cell, std::regex(INT_VALUE_REGEX))) {
        return static_cast<float>(std::stoi(cell.c_str()));
    }
    // float/double
    else if (regex_match(cell, std::regex(FLOAT_VALUE_REGEX))) {
        return std::strtof(cell.c_str(), NULL);
    }

    Hanami::toLowerCase(cell);
    if (cell == "null") {
        return 0.0f;
    }
    // true
    else if (cell == "true") {
        return 1.0f;
    }
    // false
    else if (cell == "false") {
        return 0.0f;
    }
    else {
        // ignore other lines
        return 0.0f;
    }
}

/**
 * @brief convert csv-data into generic format
 *
 * @param filePath path to the resulting file
 * @param name dataset name
 * @param inputBuffer buffer with input-data
 * @param error reference for error-output
 *
 * @return true, if successfull, else false
 */
bool
FinalizeCsvDataSetV1M0::convertCsvData(const std::string& filePath,
                                       const std::string& name,
                                       const Hanami::DataBuffer& inputBuffer,
                                       Hanami::ErrorContainer& error)
{
    // prepare content-processing
    const std::string stringContent(static_cast<char*>(inputBuffer.data),
                                    inputBuffer.usedBufferSize);

    // buffer for values to reduce write-access to file
    std::vector<float> lineBuffer;
    DataSetFileHandle fileHandle;

    // split content
    std::vector<std::string> lines;
    Hanami::splitStringByDelimiter(lines, stringContent, '\n');

    bool isHeader = true;

    for (uint64_t lineNum = 0; lineNum < lines.size(); lineNum++) {
        const std::string* line = &lines[lineNum];

        // check if the line is relevant to ignore broken lines
        const uint64_t numberOfColumns = std::count(line->begin(), line->end(), ',') + 1;
        if (numberOfColumns == 1) {
            continue;
        }

        // split line
        std::vector<std::string> lineContent;
        Hanami::splitStringByDelimiter(lineContent, *line, ',');

        if (isHeader) {
            json description;

            uint64_t counter = 0;
            for (const std::string& colName : lineContent) {
                json inputDescrEntry;
                inputDescrEntry["column_start"] = counter;
                inputDescrEntry["column_end"] = counter + 1;
                description[colName] = inputDescrEntry;
                counter++;
            }
            isHeader = false;

            // initialize file
            if (initNewDataSetFile(fileHandle,
                                   filePath,
                                   name,
                                   description,
                                   DataSetType::FLOAT_TYPE,
                                   numberOfColumns,
                                   error)
                != OK)
            {
                return false;
            }
            fileHandle.initReadWriteBuffer(10);

            lineBuffer = std::vector<float>(numberOfColumns, 0.0f);
        }
        else {
            std::fill(lineBuffer.begin(), lineBuffer.end(), 0.0f);

            for (uint64_t colNum = 0; colNum < lineContent.size(); colNum++) {
                std::string* cell = &lineContent[colNum];
                if (colNum < lineBuffer.size()) {
                    lineBuffer[colNum] = convertField(*cell);
                }
            }

            if (appendToDataSet(
                    fileHandle, &lineBuffer[0], lineBuffer.size() * sizeof(float), error)
                != OK)
            {
                return false;
            }
        }
    }

    return true;
}
