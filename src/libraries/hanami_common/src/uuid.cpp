/**
 * @file        uuid.cpp
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

#include <hanami_common/uuid.h>
#include <uuid/uuid.h>

#include <regex>
#include <string>

/**
 * @brief check if an id is an uuid
 *
 * @param id id to check
 *
 * @return true, if id is an uuid, else false
 */
bool
isUuid(const std::string& id)
{
    const std::regex uuidRegex(UUID_REGEX);
    return regex_match(id, uuidRegex);
}

/**
 * @brief generate a new uuid with external library
 *
 * @return new uuid
 */
const UUID
generateUuid()
{
    uuid_t binaryUuid;
    UUID result;

    uuid_generate_random(binaryUuid);
    uuid_unparse_lower(binaryUuid, result.uuid);

    return result;
}
