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

#ifndef HANAMI_DATASETFILE_H
#define HANAMI_DATASETFILE_H

#include <hanami_common/buffer/data_buffer.h>
#include <hanami_common/enums.h>
#include <hanami_common/files/binary_file.h>
#include <hanami_common/logger.h>
#include <hanami_common/structs.h>
#include <stdint.h>

#include <cstring>
#include <string>
#include <type_traits>
#include <vector>

enum DataSetType : uint8_t {
    UNDEFINED_TYPE = 0,
    UINT8_TYPE = 1,
    FLOAT_TYPE = 4,
};

struct DataSetFileHandle;
ReturnStatus openDataSetFile(DataSetFileHandle& result,
                             const std::string& filePath,
                             Hanami::ErrorContainer& error);

ReturnStatus initNewDataSetFile(DataSetFileHandle& result,
                                const std::string& filePath,
                                const std::string& name,
                                const json& description,
                                const DataSetType type,
                                const uint64_t numberOfColumns,
                                Hanami::ErrorContainer& error);

ReturnStatus initNewDataSetFile(const std::string& filePath,
                                const std::string& name,
                                const json& description,
                                const DataSetType type,
                                const uint64_t numberOfColumns,
                                Hanami::ErrorContainer& error);

ReturnStatus appendToDataSet(DataSetFileHandle& fileHandle,
                             const void* input,
                             const uint64_t inputSize,
                             Hanami::ErrorContainer& error);

ReturnStatus appendValueToDataSet(DataSetFileHandle& fileHandle,
                                  const float input,
                                  Hanami::ErrorContainer& error);

ReturnStatus _appendToDataSetFile(DataSetFileHandle& fileHandle,
                                  const void* input,
                                  const uint64_t inputSize,
                                  Hanami::ErrorContainer& error);

ReturnStatus _fillDataSetReadBuffer(DataSetFileHandle& fileHandle,
                                    const uint64_t newStartRow,
                                    Hanami::ErrorContainer& error);

struct DataSetSelector {
    uint64_t startRow = 0;
    uint64_t endRow = 0;
    uint64_t columnStart = 0;
    uint64_t columnEnd = 0;

    DataSetSelector() {}

    ReturnStatus fromJson(const json& input)
    {
        if (input.contains("start_row")) {
            startRow = input["start_row"];
        }
        if (input.contains("column_start")) {
            columnStart = input["column_start"];
        }

        if (input.contains("column_end") == false) {
            return INVALID_INPUT;
        }

        columnEnd = input["column_end"];

        return OK;
    }
};

struct DataSetHeader {
    const char typeIdentifier[8] = "hanami";
    const char fileIdentifier[32] = "data-set";
    const char version[8] = "v1";
    const char minorVersion[8] = "0alpha";

    DataSetType dataType = UNDEFINED_TYPE;
    uint8_t typeSize = 1;
    uint8_t padding1[2];
    uint32_t descriptionSize = 0;

    uint64_t fileSize = 0;
    uint64_t numberOfRows = 0;
    uint64_t numberOfColumns = 0;

    Hanami::NameEntry name;

    uint8_t padding2[3752];

    DataSetHeader() {}

    void toJson(json& result) const
    {
        result = json::object();
        result["version"] = version;
        result["name"] = name.getName();
        result["data_type"] = dataType;
        result["file_size"] = fileSize;
        result["number_of_rows"] = numberOfRows;
        result["number_of_columns"] = numberOfColumns;
    }
};
static_assert(sizeof(DataSetHeader) == 4096);

struct DataSetFileHandle {
    DataSetHeader header;
    json description;
    Hanami::BinaryFile* targetFile = nullptr;
    Hanami::DataBuffer writeBuffer;
    Hanami::DataBuffer readBuffer;
    DataSetSelector readSelector;
    int64_t bufferStartRow = -1;
    int64_t bufferEndRow = -1;

    DataSetFileHandle() { initReadWriteBuffer(1); }

    DataSetFileHandle(const uint64_t bufferSizeMiB) { initReadWriteBuffer(bufferSizeMiB); }

    ~DataSetFileHandle()
    {
        if (targetFile != nullptr) {
            Hanami::ErrorContainer error;
            if (writeRemainingBufferToFile(error) == false) {
                LOG_ERROR(error);
            }
            if (updateHeaderInFile(error) == false) {
                error.addMessage("Failed to update data-set header in file while closing");
                LOG_ERROR(error);
            }
            if (targetFile->closeFile(error) == false) {
                error.addMessage("Failed to close data-set-file");
                LOG_ERROR(error);
            }

            delete targetFile;
        }
    }

    DataSetFileHandle(DataSetFileHandle&& otherObf)
    {
        memcpy(&header, &otherObf.header, sizeof(header));

        targetFile = otherObf.targetFile;
        otherObf.targetFile = nullptr;

        readBuffer.clear();
        readBuffer.blockSize = otherObf.readBuffer.blockSize;
        readBuffer.data = otherObf.readBuffer.data;
        readBuffer.inUse = otherObf.readBuffer.inUse;
        readBuffer.numberOfBlocks = otherObf.readBuffer.numberOfBlocks;
        readBuffer.totalBufferSize = otherObf.readBuffer.totalBufferSize;
        readBuffer.usedBufferSize = otherObf.readBuffer.usedBufferSize;
        otherObf.readBuffer.data = nullptr;

        writeBuffer.clear();
        writeBuffer.blockSize = otherObf.writeBuffer.blockSize;
        writeBuffer.data = otherObf.writeBuffer.data;
        writeBuffer.inUse = otherObf.writeBuffer.inUse;
        writeBuffer.numberOfBlocks = otherObf.writeBuffer.numberOfBlocks;
        writeBuffer.totalBufferSize = otherObf.writeBuffer.totalBufferSize;
        writeBuffer.usedBufferSize = otherObf.writeBuffer.usedBufferSize;
        otherObf.writeBuffer.data = nullptr;

        readSelector = otherObf.readSelector;

        bufferStartRow = otherObf.bufferStartRow;
        bufferEndRow = otherObf.bufferEndRow;

        description = otherObf.description;
    }

    DataSetFileHandle& operator=(DataSetFileHandle&& otherObf)
    {
        memcpy(&header, &otherObf.header, sizeof(header));

        targetFile = otherObf.targetFile;
        otherObf.targetFile = nullptr;

        readBuffer.clear();
        readBuffer.blockSize = otherObf.readBuffer.blockSize;
        readBuffer.data = otherObf.readBuffer.data;
        readBuffer.inUse = otherObf.readBuffer.inUse;
        readBuffer.numberOfBlocks = otherObf.readBuffer.numberOfBlocks;
        readBuffer.totalBufferSize = otherObf.readBuffer.totalBufferSize;
        readBuffer.usedBufferSize = otherObf.readBuffer.usedBufferSize;
        otherObf.readBuffer.data = nullptr;

        writeBuffer.clear();
        writeBuffer.blockSize = otherObf.writeBuffer.blockSize;
        writeBuffer.data = otherObf.writeBuffer.data;
        writeBuffer.inUse = otherObf.writeBuffer.inUse;
        writeBuffer.numberOfBlocks = otherObf.writeBuffer.numberOfBlocks;
        writeBuffer.totalBufferSize = otherObf.writeBuffer.totalBufferSize;
        writeBuffer.usedBufferSize = otherObf.writeBuffer.usedBufferSize;
        otherObf.writeBuffer.data = nullptr;

        readSelector = otherObf.readSelector;

        bufferStartRow = otherObf.bufferStartRow;
        bufferEndRow = otherObf.bufferEndRow;

        description = otherObf.description;

        return *this;
    }

    bool initReadWriteBuffer(const uint64_t sizeMiB)
    {
        if (Hanami::reset_DataBuffer(readBuffer, Hanami::calcBytesToBlocks(sizeMiB * 1024 * 1024))
            == false)
        {
            return false;
        }
        readBuffer.usedBufferSize = 0;
        if (Hanami::reset_DataBuffer(writeBuffer, Hanami::calcBytesToBlocks(sizeMiB * 1024 * 1024))
            == false)
        {
            return false;
        }
        writeBuffer.usedBufferSize = 0;
        return true;
    }

    bool updateHeaderInFile(Hanami::ErrorContainer& error)
    {
        if (targetFile == nullptr) {
            return true;
        }
        if (targetFile->writeDataIntoFile(&header, 0, sizeof(DataSetHeader), error) == false) {
            error.addMessage("Failed to write data-set header to disc");
            return false;
        }
        return true;
    }

    bool writeRemainingBufferToFile(Hanami::ErrorContainer& error)
    {
        if (targetFile == nullptr) {
            return true;
        }
        if (writeBuffer.usedBufferSize > 0) {
            if (_appendToDataSetFile(*this, writeBuffer.data, writeBuffer.usedBufferSize, error)
                != OK)
            {
                error.addMessage(
                    "Failed write remaining write-buffer-data into data-set file while closing");
                return false;
            }
        }
        writeBuffer.usedBufferSize = 0;
        return true;
    }
};

/**
 * @brief append new row
 *
 * @param fileHandle handle of the data-set file, where the new data should be appended
 * @param input json-array as input with the new data to append.
 *              Must be a multiple of the number of columns in the data-set.
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
template <typename T>
ReturnStatus
appendToDataSet(DataSetFileHandle& fileHandle, const json& input, Hanami::ErrorContainer& error)
{
    if (input.is_array() == false || input.size() == 0) {
        return INVALID_INPUT;
    }

    if (input.size() % fileHandle.header.numberOfColumns != 0) {
        return INVALID_INPUT;
    }

    // convert json-data
    std::vector<T> temp;
    temp.resize(input.size());
    for (uint64_t i = 0; i < input.size(); i++) {
        if (input[i].is_number() == false) {
            return INVALID_INPUT;
        }

        temp[i] = input[i];
    }

    return appendToDataSet(fileHandle, &temp[0], temp.size() * sizeof(T), error);
}

/**
 * @brief internal function to convert the buffer to the outputW
 */
template <typename IN_T, typename OUT_T>
void
_convert(std::vector<OUT_T>& result, const DataSetFileHandle& fileHandle, const uint64_t byteOffset)
{
    const IN_T* data = static_cast<const IN_T*>(fileHandle.readBuffer.data);
    uint64_t counter = 0;
    for (uint64_t x = fileHandle.readSelector.columnStart; x < fileHandle.readSelector.columnEnd;
         x++)
    {
        result[counter] = static_cast<OUT_T>(data[(byteOffset / sizeof(IN_T)) + x]);
        counter++;
    }
}

/**
 * @brief get a section of data from the data-set
 *
 * @param result reference to list with all read values
 * @param fileHandle handle to the file and header
 * @param selector object to select a section within the dataset
 * @param error reference for error-output
 *
 * @return OK, INVALID_INPUT or ERROR
 */
template <typename T>
ReturnStatus
getDataFromDataSet(std::vector<T>& result,
                   DataSetFileHandle& fileHandle,
                   const uint64_t row,
                   Hanami::ErrorContainer& error)
{
    // check if file is open
    if (fileHandle.targetFile->isOpen() == false) {
        error.addMessage("Data-set file '" + fileHandle.targetFile->filePath + "' is not open");
        return INVALID_INPUT;
    }

    // check type
    const bool invalidType
        = fileHandle.header.dataType != UINT8_TYPE && fileHandle.header.dataType != FLOAT_TYPE;
    if (fileHandle.header.numberOfColumns == 0 || invalidType) {
        return INVALID_INPUT;
    }

    // check ranges
    if (fileHandle.readSelector.columnEnd > fileHandle.header.numberOfColumns
        || fileHandle.readSelector.columnStart > fileHandle.header.numberOfColumns)
    {
        return INVALID_INPUT;
    }
    if (fileHandle.readSelector.endRow > fileHandle.header.numberOfRows
        || fileHandle.readSelector.startRow > fileHandle.header.numberOfRows)
    {
        return INVALID_INPUT;
    }

    // check if requested row is in selected range
    if (row >= (fileHandle.readSelector.endRow) || row < (fileHandle.readSelector.startRow)) {
        return INVALID_INPUT;
    }

    // check if given buffer is big enough
    if (result.size() < fileHandle.readSelector.columnEnd - fileHandle.readSelector.columnStart) {
        return INVALID_INPUT;
    }

    // check if requested row is inside of the read-buffer. If not, then load it from file into the
    // buffer
    if (row < fileHandle.bufferStartRow || row >= fileHandle.bufferEndRow) {
        if (_fillDataSetReadBuffer(fileHandle, row, error) != OK) {
            return ERROR;
        }
    }

    // convert data from buffer to output-list
    const uint64_t byteOffset = (row - fileHandle.bufferStartRow)
                                * fileHandle.header.numberOfColumns * fileHandle.header.dataType;
    switch (fileHandle.header.dataType) {
        case UNDEFINED_TYPE:
            error.addMessage("Invalid data-type defined in file '" + fileHandle.targetFile->filePath
                             + "'");
            return ERROR;
        case UINT8_TYPE:
            _convert<uint8_t, T>(result, fileHandle, byteOffset);
            break;
        case FLOAT_TYPE:
            _convert<float, T>(result, fileHandle, byteOffset);
            break;
    }

    return OK;
}

#endif  // HANAMI_DATASETFILE_H
