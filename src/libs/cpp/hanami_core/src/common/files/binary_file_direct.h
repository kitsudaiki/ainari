/**
 * @file    binary_file_direct.h
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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

#ifndef BINARY_FILE_DIRECT_H
#define BINARY_FILE_DIRECT_H

#include <assert.h>
#include <fcntl.h>
#include <src/common/buffer/data_buffer.h>
#include <src/common/logger.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

namespace Hanami
{

class BinaryFileDirect
{
   public:
    BinaryFileDirect(const std::string& filePath);
    ~BinaryFileDirect();

    bool isOpen() const;
    bool allocateStorage(const uint64_t numberOfBlocks,
                         const uint32_t blockSize,
                         ErrorContainer& error);
    bool updateFileSize(ErrorContainer& error);

    bool readCompleteFile(DataBuffer& buffer, ErrorContainer& error);
    bool writeCompleteFile(DataBuffer& buffer, ErrorContainer& error);

    bool readSegment(DataBuffer& buffer,
                     const uint64_t startBlockInFile,
                     const uint64_t numberOfBlocks,
                     const uint64_t startBlockInBuffer,
                     ErrorContainer& error);
    bool writeSegment(DataBuffer& buffer,
                      const uint64_t startBlockInFile,
                      const uint64_t numberOfBlocks,
                      const uint64_t startBlockInBuffer,
                      ErrorContainer& error);

    bool closeFile(ErrorContainer& error);

    // public variables to avoid stupid getter
    uint64_t m_totalFileSize = 0;
    std::string m_filePath = "";

   private:
    int m_fileDescriptor = -1;
    uint16_t m_blockSize = 512;

    bool initFile(ErrorContainer& error);
    bool allocateStorage(const uint64_t numberOfBytes, ErrorContainer& error);
};

}  // namespace Hanami

#endif  // BINARY_FILE_DIRECT_H
