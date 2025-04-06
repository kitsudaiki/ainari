/**
 * @file        dataset_file_io.cpp
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

#include "dataset_file_io.h"

/**
 * @brief internal function to append new data to the data-set-file
 *
 * @param fileHandle handle of the data-set file, where the new data should be appended
 * @param input new data to append to the data-set
 * @paran inputSize number of bytes of the new input
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
_appendToDataSetFile(DataSetFileHandle& fileHandle,
                     const void* input,
                     const uint64_t inputSize,
                     Hanami::ErrorContainer& error)
{
    const uint64_t offset = fileHandle.targetFile->fileSize;
    if (fileHandle.targetFile->allocateStorage(inputSize, error) == false) {
        return ERROR;
    }

    if (fileHandle.targetFile->writeDataIntoFile(input, offset, inputSize, error) == false) {
        return ERROR;
    }

    fileHandle.header.fileSize = fileHandle.targetFile->fileSize;
    fileHandle.header.numberOfRows
        += (inputSize / fileHandle.header.typeSize) / fileHandle.header.numberOfColumns;

    return OK;
}

/**
 * @brief append new data
 *
 * @param fileHandle handle of the data-set file, where the new data should be appended
 * @param input new data to append to the data-set
 * @paran inputSize number of bytes of the new input
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
appendToDataSet(DataSetFileHandle& fileHandle,
                const void* input,
                const uint64_t inputSize,
                Hanami::ErrorContainer& error)
{
    // check if new data still fit into the write-buffer
    if (fileHandle.writeBuffer.usedBufferSize + inputSize > fileHandle.writeBuffer.totalBufferSize)
    {
        // write data from write-buffer into file, if exist
        if (fileHandle.writeBuffer.usedBufferSize > 0) {
            ReturnStatus ret = _appendToDataSetFile(fileHandle,
                                                    fileHandle.writeBuffer.data,
                                                    fileHandle.writeBuffer.usedBufferSize,
                                                    error);
            if (ret != OK) {
                return ret;
            }
            fileHandle.writeBuffer.usedBufferSize = 0;
        }

        // check if the new data even fit into the complete write-buffer
        if (inputSize > fileHandle.writeBuffer.totalBufferSize) {
            ReturnStatus ret = _appendToDataSetFile(fileHandle, input, inputSize, error);
            if (ret != OK) {
                return ret;
            }
            fileHandle.writeBuffer.usedBufferSize = 0;
        }
        else {
            Hanami::addData_DataBuffer(fileHandle.writeBuffer, input, inputSize);
        }
    }
    else {
        Hanami::addData_DataBuffer(fileHandle.writeBuffer, input, inputSize);
    }

    return OK;
}

/**
 * @brief append single value
 *
 * @param fileHandle handle of the data-set file, where the new data should be appended
 * @param input single value to append to the data-set
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
appendValueToDataSet(DataSetFileHandle& fileHandle,
                     const float input,
                     Hanami::ErrorContainer& error)
{
    return appendToDataSet(fileHandle, &input, 4, error);
}

/**
 * @brief open a data-set file
 *
 * @param result resulting handle to the open file
 * @param filePath path on the local disc to the data-set, which should be opend
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
openDataSetFile(DataSetFileHandle& result,
                const std::string& filePath,
                Hanami::ErrorContainer& error)
{
    // check source
    if (std::filesystem::exists(filePath) == false) {
        error.addMessage("Data-set file '" + filePath + "' doesn't exist.");
        return INVALID_INPUT;
    }

    // open file
    result.targetFile = new Hanami::BinaryFile(filePath);
    if (result.targetFile->isOpen() == false) {
        error.addMessage("Failed to open data-set file '" + filePath + "'.");
        return ERROR;
    }

    // check for minimum size
    if (result.targetFile->fileSize < sizeof(DataSetHeader)) {
        error.addMessage("Data-set file '" + filePath + "' is too small for the header.");
        return INVALID_INPUT;
    }

    // read header of file to identify type and get description-info
    if (result.targetFile->readDataFromFile(&result.header, 0, sizeof(DataSetHeader), error)
        == false)
    {
        error.addMessage("Failed to read header of data-set-file");
        return ERROR;
    }

    // read description from file
    Hanami::DataBuffer descriptionBuffer(Hanami::calcBytesToBlocks(result.header.descriptionSize));
    if (result.targetFile->readDataFromFile(
            descriptionBuffer.data, sizeof(DataSetHeader), result.header.descriptionSize, error)
        == false)
    {
        error.addMessage("Failed to read description of data-set-file");
        return ERROR;
    }

    // parse description
    const std::string description
        = std::string(static_cast<char*>(descriptionBuffer.data), result.header.descriptionSize);
    try {
        result.description = json::parse(description);
    }
    catch (const json::parse_error& ex) {
        error.addMessage("Invalid dataset-description '" + description + "'");
        return INVALID_INPUT;
    }

    return OK;
}

/**
 * @brief create a new data-set file
 *
 * @param result handle to the new created file
 * @param filePath path on the local disc, where the file should be created
 * @param name name of the new data-set
 * @param description description of the content
 * @param type tpe of the new data-set
 * @param numberOfColumns maximum number of columns, which should be stored by the data-set
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
initNewDataSetFile(DataSetFileHandle& result,
                   const std::string& filePath,
                   const std::string& name,
                   const json& description,
                   const DataSetType type,
                   const uint64_t numberOfColumns,
                   Hanami::ErrorContainer& error)
{
    if (type == UNDEFINED_TYPE) {
        return INVALID_INPUT;
    }

    const std::string descriptionStr = description.dump();
    result.header.descriptionSize = descriptionStr.size();

    // check source
    if (std::filesystem::exists(filePath) == true) {
        error.addMessage("Data-set file '" + filePath + "' already exist.");
        return INVALID_INPUT;
    }

    // open file
    result.targetFile = new Hanami::BinaryFile(filePath);
    if (result.targetFile->isOpen() == false) {
        error.addMessage("Failed to open data-set file '" + filePath + "'.");
        return ERROR;
    }

    // allocate initaial storage for the header
    if (result.targetFile->allocateStorage(sizeof(DataSetHeader) + result.header.descriptionSize,
                                           error)
        == false)
    {
        error.addMessage("Failed to allocate storage in file '" + filePath
                         + "' for the data-set header");
        return ERROR;
    }

    // set name in header
    if (result.header.name.setName(name) == false) {
        error.addMessage("New data-set name '" + name + "' is invalid");
        return INVALID_INPUT;
    }

    // write description to target
    if (result.targetFile->writeDataIntoFile(
            descriptionStr.c_str(), sizeof(DataSetHeader), result.header.descriptionSize, error)
        == false)
    {
        error.addMessage("Failed to write dataset-description to disc");
        return ERROR;
    }

    // write header to target
    result.header.dataType = type;
    result.header.numberOfColumns = numberOfColumns;
    result.header.typeSize = type;

    if (result.updateHeaderInFile(error) == false) {
        error.addMessage("Failed to update data-set header in file");
        return ERROR;
    }
    result.header.fileSize = result.targetFile->fileSize;
    result.description = description;

    return OK;
}

/**
 * @brief create a new data-set file
 *
 * @param filePath path on the local disc, where the file should be created
 * @param name name of the new data-set
 * @param description description of the content
 * @param type tpe of the new data-set
 * @param numberOfColumns maximum number of columns, which should be stored by the data-set
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
initNewDataSetFile(const std::string& filePath,
                   const std::string& name,
                   const json& description,
                   const DataSetType type,
                   const uint64_t numberOfColumns,
                   Hanami::ErrorContainer& error)
{
    if (type == UNDEFINED_TYPE) {
        return INVALID_INPUT;
    }

    // check source
    if (std::filesystem::exists(filePath) == true) {
        error.addMessage("Data-set file '" + filePath + "' already exist.");
        return ERROR;
    }

    DataSetHeader header;
    Hanami::BinaryFile targetFile(filePath);

    const std::string descriptionStr = description.dump();
    header.descriptionSize = descriptionStr.size();

    // open file
    if (targetFile.isOpen() == false) {
        error.addMessage("Failed to open data-set file '" + filePath + "'.");
        return ERROR;
    }

    // allocate initaial storage for the header
    if (targetFile.allocateStorage(sizeof(DataSetHeader) + header.descriptionSize, error) == false)
    {
        error.addMessage("Failed to allocate storage in file '" + filePath
                         + "' for the data-set header");
        return ERROR;
    }

    // set name in header
    if (header.name.setName(name) == false) {
        error.addMessage("New data-set name '" + name + "' is invalid");
        return INVALID_INPUT;
    }

    // write description to target
    if (targetFile.writeDataIntoFile(
            descriptionStr.c_str(), sizeof(DataSetHeader), header.descriptionSize, error)
        == false)
    {
        error.addMessage("Failed to write dataset-description to disc");
        return ERROR;
    }

    // write header to target
    header.dataType = type;
    header.numberOfColumns = numberOfColumns;
    header.typeSize = type;
    header.fileSize = targetFile.fileSize;

    if (targetFile.writeDataIntoFile(&header, 0, sizeof(DataSetHeader), error) == false) {
        error.addMessage("Failed to update data-set header in file");
        return ERROR;
    }

    return OK;
}

/**
 * @brief fill the read-buffer of the file-handle with a new chunk of data from the file
 *
 * @param fileHandle handle of the data-set file, where the new data should be read
 * @param newStartRow start-row to load into the buffer
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
ReturnStatus
_fillDataSetReadBuffer(DataSetFileHandle& fileHandle,
                       const uint64_t newStartRow,
                       Hanami::ErrorContainer& error)
{
    if (newStartRow > fileHandle.header.numberOfRows) {
        return INVALID_INPUT;
    }

    // read block from file into buffer
    const uint64_t rowByteCount = fileHandle.header.numberOfColumns * fileHandle.header.dataType;
    const uint64_t byteOffset
        = sizeof(DataSetHeader) + fileHandle.header.descriptionSize + (newStartRow * rowByteCount);

    uint64_t numberOfRowsForBuffer = fileHandle.readBuffer.totalBufferSize / rowByteCount;
    if (numberOfRowsForBuffer > fileHandle.readSelector.endRow - newStartRow) {
        numberOfRowsForBuffer = fileHandle.readSelector.endRow - newStartRow;
    }
    fileHandle.readBuffer.usedBufferSize = numberOfRowsForBuffer * rowByteCount;

    if (fileHandle.targetFile->readDataFromFile(
            fileHandle.readBuffer.data, byteOffset, fileHandle.readBuffer.usedBufferSize, error)
        == false)
    {
        error.addMessage("Failed to read data of dataset-file '" + fileHandle.targetFile->filePath
                         + "'");
        return ERROR;
    }

    fileHandle.bufferStartRow = newStartRow;
    fileHandle.bufferEndRow = newStartRow + numberOfRowsForBuffer;

    return OK;
}
