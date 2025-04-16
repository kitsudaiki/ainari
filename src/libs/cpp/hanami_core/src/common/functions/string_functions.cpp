/**
 * @file       string_functions.cpp
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

#include <src/common/functions/string_functions.h>

#include <algorithm>
#include <sstream>

namespace Hanami
{

/**
 * @brief sptrit a string at a specific delimiter
 *
 * @param result reference to the resulting list with the splitted content
 * @param inputString string, which sould be splitted
 * @param delim delimiter to identify the points to split
 */
void
splitStringByDelimiter(std::vector<std::string>& result,
                       const std::string& inputString,
                       const char delim)
{
    // precheck
    if (inputString.length() == 0) {
        return;
    }

    // clear list of results, if necessary
    if (result.size() > 0) {
        result.clear();
    }

    // count number of final parts, to allocate memory in vector only one time
    uint64_t numberOfSubstrings = std::count(inputString.begin(), inputString.end(), delim);
    numberOfSubstrings += 1;
    result.reserve(numberOfSubstrings);

    // init variables
    std::stringstream inputStream(inputString);
    std::string item;

    // split
    while (std::getline(inputStream, item, delim)) {
        result.emplace_back(item);
    }

    return;
}

/**
 * @brief split a string into part with a maximum length
 *
 * @param result reference to the resulting list with the splitted content
 * @param inputString string, which sould be splitted
 * @param splitLength max length of the single substrings
 */
void
splitStringByLength(std::vector<std::string>& result,
                    const std::string& inputString,
                    const uint64_t splitLength)
{
    // clear list of results, if necessary
    if (result.size() > 0) {
        result.clear();
    }

    // calculate number of splits
    const uint64_t numberOfSubstrings = (inputString.length() / splitLength) + 1;

    // allocate memory, to make things faster
    result.reserve(numberOfSubstrings);

    // split string
    for (uint64_t i = 0; i < numberOfSubstrings; i++) {
        result.emplace_back(inputString.substr(i * splitLength, splitLength));
    }

    return;
}

/**
 * @brief replace a substring with another substring
 *
 * @param original original string, which should be changed
 * @param oldSubstring old substring, which should be replace
 * @param newSubstring new substring to replace the old one
 */
void
replaceSubstring(std::string& original,
                 const std::string& oldSubstring,
                 const std::string& newSubstring)
{
    std::string::size_type pos = 0u;
    while ((pos = original.find(oldSubstring, pos)) != std::string::npos) {
        original.replace(pos, oldSubstring.length(), newSubstring);
        pos += newSubstring.length();
    }
}

/**
 * @brief remove whitespaces from a string
 *
 * @param input input-string, from which the whitespaces should be removed
 */
void
removeWhitespaces(std::string& input)
{
    input.erase(std::remove_if(input.begin(), input.end(), isspace), input.end());
}

/**
 * @brief trim string on the left side
 *
 * @param original string, which should be changed
 * @param chars chars to remove
 */
void
ltrim(std::string& original, const std::string& chars)
{
    original.erase(0, original.find_first_not_of(chars));
}

/**
 * @brief trim string on the right side
 *
 * @param original string, which should be changed
 * @param chars chars to remove
 */
void
rtrim(std::string& original, const std::string& chars)
{
    original.erase(original.find_last_not_of(chars) + 1);
}

/**
 * @brief trim string on both sides
 *
 * @param original string, which should be changed
 * @param chars chars to remove
 */
void
trim(std::string& original, const std::string& chars)
{
    ltrim(original, chars);
    rtrim(original, chars);
}

/**
 * @brief converts string to upper-case
 *
 * @param original reference to string, which have to be converted
 */
void
toUpperCase(std::string& original)
{
    std::transform(original.begin(),
                   original.end(),
                   original.begin(),
                   [](unsigned char c) { return std::toupper(c); });
}

/**
 * @brief converts string to lower-case
 *
 * @param original reference to string, which have to be converted
 */
void
toLowerCase(std::string& original)
{
    std::transform(original.begin(),
                   original.end(),
                   original.begin(),
                   [](unsigned char c) { return std::tolower(c); });
}

}  // namespace Hanami
