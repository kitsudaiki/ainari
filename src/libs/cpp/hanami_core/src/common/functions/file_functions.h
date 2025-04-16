/**
 * @file       file_functions.h
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
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

#ifndef FILE_FUNCTIONS_H
#define FILE_FUNCTIONS_H

#include <assert.h>
#include <src/common/logger.h>

#include <filesystem>
#include <string>
#include <vector>

namespace Hanami
{

bool listFiles(std::vector<std::string>& fileList,
               const std::string& path,
               const bool withSubdirs = true,
               const std::vector<std::string>& exceptions = {});

bool renameFileOrDir(const std::filesystem::path& oldPath,
                     const std::filesystem::path& newPath,
                     ErrorContainer& error);
bool copyPath(const std::filesystem::path& sourcePath,
              const std::filesystem::path& targetPath,
              ErrorContainer& error,
              const bool force = true);
bool createDirectory(const std::filesystem::path& path, ErrorContainer& error);
bool deleteFileOrDir(const std::filesystem::path& path, ErrorContainer& error);

}  // namespace Hanami

#endif  // FILE_FUNCTIONS_H
