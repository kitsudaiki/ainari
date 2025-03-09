/**
 * @file        file_upload.h
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

#ifndef FILE_UPLOAD_H
#define FILE_UPLOAD_H

#include <hanami_common/structs.h>

#include "hanami_common/upload_file_handle.h"

class HttpWebsocketThread;

bool recvFileUploadPackage(Hanami::UploadFileHandle* fileHandle,
                           const void* data,
                           const uint64_t dataSize,
                           std::string& errorMessage);
void sendFileUploadResponse(HttpWebsocketThread* msgClient,
                            const bool success,
                            const std::string& errorMessage);

#endif  // FILE_UPLOAD_H
