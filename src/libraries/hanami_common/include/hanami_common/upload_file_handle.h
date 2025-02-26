/**
 * @file        upload_file_handle.h
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

#ifndef UPLOAD_FILE_HANDLE_H
#define UPLOAD_FILE_HANDLE_H

#include <stdint.h>

#include <string>

#include "buffer/bit_buffer.h"
#include "files/binary_file.h"
#include "logger.h"
#include "structs.h"

#define UNINTI_POINT_32 0x0FFFFFFF
#define TRANSFER_SEGMENT_SIZE (128 * 1024)

namespace Hanami
{

//==================================================================================================

struct UploadFileHandle {
    Hanami::UserContext userContext;
    Hanami::BinaryFile* file = nullptr;
    Hanami::BitBuffer* bitBuffer = nullptr;
    uint64_t timeoutCounter = 0;
    bool lock = false;

    UploadFileHandle() {}

    UploadFileHandle(const UploadFileHandle& other)
    {
        userContext = other.userContext;
        file = other.file;
        bitBuffer = other.bitBuffer;
    }

    ~UploadFileHandle()
    {
        if (file != nullptr) {
            delete file;
        }
        if (bitBuffer != nullptr) {
            delete bitBuffer;
        }
    }

    UploadFileHandle& operator=(const UploadFileHandle& other)
    {
        userContext = other.userContext;
        file = other.file;
        bitBuffer = other.bitBuffer;
        return *this;
    }

    /**
     * @brief add data to the buffer of the file-handle
     *
     * @param pos byte-postion where to add data to the buffer
     * @param data data to write
     * @param size number of byte to write
     * @param errorMessage reference for error-output
     *
     * @return true, if successful, else false
     */
    bool addDataToPos(const uint64_t pos,
                      const void* data,
                      const uint64_t size,
                      std::string& errorMessage)
    {
        Hanami::ErrorContainer error;

        // write data to file
        if (file->writeDataIntoFile(data, pos, size, error) == false) {
            LOG_ERROR(error);
            errorMessage = "Failed to write data to disc";
            return false;
        }

        // update progress in bit-buffer
        bitBuffer->set(pos / TRANSFER_SEGMENT_SIZE, true);

        return true;
    }
};

//==================================================================================================

}  // namespace Hanami

#endif  // UPLOAD_FILE_HANDLE_H
