/**
 * @file    binary_file.h
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

#ifndef BINARY_FILE_H
#define BINARY_FILE_H

#include <assert.h>
#include <fcntl.h>
#include <src/common/buffer/data_buffer.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#include <string>

namespace Hanami
{

class BinaryFile
{
   public:
    BinaryFile(const std::string& filePath);
    ~BinaryFile();

    bool isOpen() const;
    bool allocateStorage(const uint64_t numberOfBytes, std::string& error);
    bool updateFileSize(std::string& error);

    bool readCompleteFile(DataBuffer& buffer, std::string& error);
    bool writeCompleteFile(DataBuffer& buffer, std::string& error);

    bool writeDataIntoFile(const void* data,
                           const uint64_t startBytePosition,
                           const uint64_t numberOfBytes,
                           std::string& error);
    bool readDataFromFile(void* data,
                          const uint64_t startBytePosition,
                          const uint64_t numberOfBytes,
                          std::string& error);

    bool closeFile(std::string& error);

    // public variables to avoid stupid getter
    uint64_t fileSize = 0;
    std::string filePath = "";

   private:
    int m_fileDescriptor = -1;

    bool initFile(std::string& error);
};

}  // namespace Hanami

#endif  // BINARY_FILE_H
