/**
 * @file       string_functions.h
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

#ifndef STRING_FUNCTIONS_H
#define STRING_FUNCTIONS_H

#include <stdint.h>

#include <string>
#include <vector>

namespace Hanami
{

void splitStringByDelimiter(std::vector<std::string>& result,
                            const std::string& inputString,
                            const char delim);
void splitStringByLength(std::vector<std::string>& result,
                         const std::string& inputString,
                         const uint64_t splitLength);
void replaceSubstring(std::string& original,
                      const std::string& oldSubstring,
                      const std::string& newSubstring);
void removeWhitespaces(std::string& input);

void ltrim(std::string& original, const std::string& chars = "\t\n\v\f\r ");
void rtrim(std::string& original, const std::string& chars = "\t\n\v\f\r ");
void trim(std::string& original, const std::string& chars = "\t\n\v\f\r ");

void toUpperCase(std::string& original);
void toLowerCase(std::string& original);

}  // namespace Hanami

#endif  // STRINGFUNCTIONS_H
