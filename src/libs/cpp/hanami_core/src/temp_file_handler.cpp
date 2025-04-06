/**
 * @file        temp_file_handler.cpp
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

#include "temp_file_handler.h"

#include <database/tempfile_table.h>
#include <hanami_common/functions/file_functions.h>
#include <hanami_common/uuid.h>
#include <hanami_config/config_handler.h>

TempFileHandler* TempFileHandler::instance = nullptr;

TempFileHandler::TempFileHandler() : Hanami::Thread("tempfile-handler") {}

TempFileHandler::~TempFileHandler() {}

/**
 * @brief initialize new temporary file
 *
 * @param uuid uuid of the new temporary file
 * @param name name of the file for easier identification
 * @param relatedUuid uuid of the related resource
 * @param size number of bytes of the file, to allocate the storage on disc
 * @param userContext user-context for database-access
 * @param error reference for error-output
 *
 * @return OK if found, INVALID_INPUT if not found, ERROR in case of internal error
 */
ReturnStatus
TempFileHandler::initNewFile(std::string& uuid,
                             const std::string& name,
                             const std::string& relatedUuid,
                             const uint64_t size,
                             const Hanami::UserContext& userContext,
                             Hanami::ErrorContainer& error)
{
    const std::lock_guard<std::mutex> lock(m_fileHandleMutex);

    uuid = generateUuid().toString();

    bool success = false;
    std::filesystem::path targetFilePath
        = GET_STRING_CONFIG("storage", "tempfile_location", success);
    targetFilePath = targetFilePath / std::filesystem::path(uuid);

    ReturnStatus result = ERROR;

    Hanami::BinaryFile* tempfile = nullptr;
    do {
        // allocate storage on disc
        tempfile = new Hanami::BinaryFile(targetFilePath);
        if (tempfile->allocateStorage(size, error) == false) {
            break;
        }

        // calculate number of segments
        uint64_t numberOfInputSegments = size / TRANSFER_SEGMENT_SIZE;
        if (numberOfInputSegments % TRANSFER_SEGMENT_SIZE == 0) {
            numberOfInputSegments++;
        }

        // create file-handle object
        Hanami::UploadFileHandle fileHandle;
        fileHandle.userContext = userContext;
        fileHandle.file = tempfile;
        fileHandle.bitBuffer = new Hanami::BitBuffer(numberOfInputSegments);

        auto ret = m_tempFiles.try_emplace(uuid, std::move(fileHandle));
        if (ret.second == false) {
            error.addMessage("UUID '" + uuid + "' already exist in tempfiles");
            break;
        }
        else {
            // set pointer to default again to avoid a try of double-free
            fileHandle.bitBuffer = nullptr;
            fileHandle.file = nullptr;
        }

        TempfileTable::TempfileDbEntry dbEntry;
        dbEntry.uuid = uuid;
        dbEntry.name = name;
        dbEntry.projectId = userContext.projectId;
        dbEntry.ownerId = userContext.userId;
        dbEntry.visibility = "private";
        dbEntry.fileSize = size;
        dbEntry.location = targetFilePath;
        dbEntry.relatedResourceType = "dataset";
        dbEntry.relatedResourceUuid = relatedUuid;

        // create database-endtry
        result = TempfileTable::getInstance()->addTempfile(dbEntry, userContext, error);
        if (result != OK) {
            error.addMessage("Failed to add tempfile-entry with UUID '" + uuid + "' to database");
            LOG_ERROR(error);
            break;
        }

        result = OK;
        break;
    }
    while (true);

    // cleanup if failed
    if (result != OK) {
        auto it = m_tempFiles.find(uuid);
        if (it != m_tempFiles.end()) {
            m_tempFiles.erase(it);
        }

        TempfileTable::getInstance()->deleteTempfile(uuid, userContext, error);
    }

    return result;
}

/**
 * @brief get handler of a temporaray file
 *
 * @param uuid uuid of the temporary file
 *
 * @return pointer to handle if uuid found, else nullptr
 */
Hanami::UploadFileHandle*
TempFileHandler::getFileHandle(const std::string& uuid, const Hanami::UserContext& context)
{
    const std::lock_guard<std::mutex> lock(m_fileHandleMutex);

    const auto it = m_tempFiles.find(uuid);
    if (it != m_tempFiles.end()) {
        if (context.userId == it->second.userContext.userId
            && context.projectId == it->second.userContext.projectId)
        {
            return &it->second;
        }
    }

    return nullptr;
}

/**
 * @brief add data to the temporary file
 *
 * @param uuid uuid of the temporary file
 * @param pos position in the file where to add the data
 * @param data pointer to the data to add
 * @param size size of the data to add
 *
 * @return false, if id not found, else true
 */
bool
TempFileHandler::addDataToPos(const std::string& uuid,
                              const uint64_t pos,
                              const void* data,
                              const uint64_t size)
{
    const std::lock_guard<std::mutex> lock(m_fileHandleMutex);

    Hanami::ErrorContainer error;

    const auto it = m_tempFiles.find(uuid);
    if (it == m_tempFiles.end()) {
        error.addMessage("File with UUID '" + uuid + "' is unknown.");
        LOG_ERROR(error);
        return false;
    }

    // write data to file
    Hanami::BinaryFile* ptr = it->second.file;
    if (ptr->writeDataIntoFile(data, pos, size, error) == false) {
        LOG_ERROR(error);
        return false;
    }

    // update progress in bit-buffer
    it->second.bitBuffer->set(pos / TRANSFER_SEGMENT_SIZE, true);

    return true;
}

/**
 * @brief get data from the temporary file
 *
 * @param result data-buffer for the resulting data of the file
 * @param uuid uuid of the temporary file
 *
 * @return false, if id not found, else true
 */
bool
TempFileHandler::getData(Hanami::DataBuffer& result, const std::string& uuid)
{
    const std::lock_guard<std::mutex> lock(m_fileHandleMutex);

    Hanami::ErrorContainer error;

    const auto it = m_tempFiles.find(uuid);
    if (it != m_tempFiles.end()) {
        Hanami::BinaryFile* ptr = it->second.file;
        return ptr->readCompleteFile(result, error);
    }

    return false;
}

/**
 * @brief remove an uuid from this class and database and delete the file within the storage
 *
 * @param uuid uuid of the temporary file
 * @param userContext user-context for database-access
 * @param error reference for error-output
 *
 * @return false, if uuid not found, else true
 */
bool
TempFileHandler::removeData(const std::string& uuid,
                            const Hanami::UserContext& userContext,
                            Hanami::ErrorContainer& error)
{
    const std::lock_guard<std::mutex> lock(m_fileHandleMutex);

    if (removeTempfile(uuid, userContext, error) == false) {
        return false;
    }

    // close file and remove from tempfile-handler
    const auto it = m_tempFiles.find(uuid);
    if (it != m_tempFiles.end()) {
        m_tempFiles.erase(it);
    }

    return true;
}

/**
 * @brief remove an uuid from this class and database and delete the file within the storage
 *
 * @param uuid uuid of the temporary file
 * @param userContext user-context for database-access
 * @param error reference for error-output
 *
 * @return false, if uuid not found, else true
 */
bool
TempFileHandler::removeTempfile(const std::string& uuid,
                                const Hanami::UserContext& userContext,
                                Hanami::ErrorContainer& error)
{
    // check tempfile-database form entry
    TempfileTable::TempfileDbEntry tempfileData;
    ReturnStatus ret
        = TempfileTable::getInstance()->getTempfile(tempfileData, uuid, userContext, error);
    if (ret == INVALID_INPUT) {
        error.addMessage("Tempfile with '" + uuid + "' not found in database");
        return false;
    }
    if (ret == ERROR) {
        error.addMessage("Internal error");
        return false;
    }

    // delete file from disc
    const std::string targetFilePath = tempfileData.location;
    if (Hanami::deleteFileOrDir(targetFilePath, error) == false) {
        error.addMessage("Failed to delete file '" + targetFilePath + "' from disc");
        LOG_ERROR(error);
    }

    // delete from tempfile-database
    ret = TempfileTable::getInstance()->deleteTempfile(uuid, userContext, error);
    if (ret == INVALID_INPUT) {
        error.addMessage("Tempfile with '" + uuid + "' not found in database");
        return false;
    }
    if (ret == ERROR) {
        error.addMessage("Internal error");
        return false;
    }

    return true;
}

/**
 * @brief move tempfile to its target-location after tempfile was finished
 *
 * @param uuid uuid of the tempfile, which should be moved
 * @param targetLocation target-location on the local direct, where to move the file to
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
TempFileHandler::moveData(const std::string& uuid,
                          const std::string& targetLocation,
                          const Hanami::UserContext& userContext,
                          Hanami::ErrorContainer& error)
{
    const std::lock_guard<std::mutex> lock(m_fileHandleMutex);

    // get location of the tempfile of the uuid
    TempfileTable::TempfileDbEntry tempfileData;
    const ReturnStatus ret
        = TempfileTable::getInstance()->getTempfile(tempfileData, uuid, userContext, error);
    if (ret == INVALID_INPUT) {
        error.addMessage("Tempfile with '" + uuid + "' not found in database");
        return false;
    }
    if (ret == ERROR) {
        error.addMessage("Internal error");
        return false;
    }

    const std::filesystem::path tempfileLocation(tempfileData.location);

    const auto it = m_tempFiles.find(uuid);
    if (it != m_tempFiles.end()) {
        Hanami::BinaryFile* ptr = it->second.file;
        if (ptr->closeFile(error) == false) {
            return false;
        }

        if (Hanami::renameFileOrDir(tempfileLocation, targetLocation, error) == false) {
            error.addMessage("Failed to move temp-file with uuid '" + uuid
                             + "' to target-location '" + targetLocation + "'");
            return false;
        }

        delete ptr;
        m_tempFiles.erase(it);

        return true;
    }

    error.addMessage("Failed to move temp-file with uuid '" + uuid
                     + ", because it can not be found.");

    return false;
}

/**
 * @brief TempFileHandler::run
 */
void
TempFileHandler::run()
{
    uint64_t timeout = 10;
    std::chrono::seconds interval(60);
    std::vector<std::string> deleteList;

    while (m_abort == false) {
        std::this_thread::sleep_for(interval);

        m_fileHandleMutex.lock();

        for (auto& [uuid, fileHandle] : m_tempFiles) {
            if (fileHandle.lock == false) {
                fileHandle.timeoutCounter++;
                if (fileHandle.timeoutCounter == timeout) {
                    Hanami::ErrorContainer error;
                    if (removeTempfile(uuid, fileHandle.userContext, error) == false) {
                        LOG_ERROR(error);
                    }
                    deleteList.push_back(uuid);
                }
            }
        }

        for (const std::string& uuid : deleteList) {
            auto it = m_tempFiles.find(uuid);
            m_tempFiles.erase(it);
        }
        deleteList.clear();

        m_fileHandleMutex.unlock();
    }
}
