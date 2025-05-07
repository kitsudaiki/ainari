/**
 * @file        temp_file_handler.h
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

#ifndef HANAMI_TEMPFILEHANDLER_H
#define HANAMI_TEMPFILEHANDLER_H

#include <src/common/buffer/bit_buffer.h>
#include <src/common/files/binary_file.h>
#include <src/common/logger.h>
#include <src/common/structs.h>
#include <src/common/threading/thread.h>
#include <src/common/upload_file_handle.h>

#include <map>
#include <string>

namespace Hanami
{
class BinaryFile;
struct DataBuffer;
}  // namespace Hanami

class TempFileHandler : Hanami::Thread
{
   public:
    static TempFileHandler* getInstance()
    {
        if (instance == nullptr) {
            instance = new TempFileHandler();
            instance->spinUnlock();
        }
        return instance;
    }
    ~TempFileHandler();

    ReturnStatus initNewFile(std::string& uuid,
                             const std::string& name,
                             const std::string& relatedUuid,
                             const uint64_t size,
                             const Hanami::UserContext& userContext,
                             Hanami::ErrorContainer& error);

    Hanami::UploadFileHandle* getFileHandle(const std::string& uuid,
                                            const Hanami::UserContext& context);

    bool addDataToPos(const std::string& uuid,
                      const uint64_t pos,
                      const void* data,
                      const uint64_t size);
    bool getData(Hanami::DataBuffer& result, const std::string& uuid);
    bool removeData(const std::string& uuid,
                    const Hanami::UserContext& userContext,
                    Hanami::ErrorContainer& error);
    bool moveData(const std::string& uuid,
                  const std::string& targetLocation,
                  const Hanami::UserContext& userContext,
                  Hanami::ErrorContainer& error);

   protected:
    void run();

   private:
    TempFileHandler();
    static TempFileHandler* instance;

    bool removeTempfile(const std::string& uuid,
                        const Hanami::UserContext& userContext,
                        Hanami::ErrorContainer& error);

    std::mutex m_fileHandleMutex;
    std::map<std::string, Hanami::UploadFileHandle> m_tempFiles;
};

#endif  // HANAMI_TEMPFILEHANDLER_H
