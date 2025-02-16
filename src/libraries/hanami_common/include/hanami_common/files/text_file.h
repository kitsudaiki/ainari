/**
 * @file    text_file.h
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

#ifndef TEXT_FILE_H
#define TEXT_FILE_H

#include <hanami_common/logger.h>

#include <string>

namespace Hanami
{

bool readFile(std::string& readContent, const std::string& filePath, ErrorContainer& error);

bool writeFile(const std::string& filePath,
               const std::string& content,
               ErrorContainer& error,
               const bool force = true);

bool appendText(const std::string& filePath, const std::string& newText, ErrorContainer& error);

bool replaceLine(const std::string& filePath,
                 const uint32_t lineNumber,
                 const std::string& newLineContent,
                 ErrorContainer& error);

bool replaceContent(const std::string& filePath,
                    const std::string& oldContent,
                    const std::string& newContent,
                    ErrorContainer& error);

}  // namespace Hanami

#endif  // TEXT_FILE_H
