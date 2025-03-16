/**
 * @file        create_user_v1_0.cpp
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

#include "create_user_v1_0.h"

#include <database/users_table.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_crypto/hashes.h>
#include <hanami_root.h>

/**
 * @brief constructor
 */
CreateUserV1M0::CreateUserV1M0() : Blossom("Register a new user within Misaki.")
{
    errorCodes.push_back(UNAUTHORIZED_RTYPE);
    errorCodes.push_back(CONFLICT_RTYPE);

    //----------------------------------------------------------------------------------------------
    // input
    //----------------------------------------------------------------------------------------------

    registerInputField("id", SAKURA_STRING_TYPE)
        .setComment("ID of the new user.")
        .setLimit(4, 254)
        .setRegex(ID_EXT_REGEX);

    registerInputField("name", SAKURA_STRING_TYPE)
        .setComment("Name of the new user.")
        .setLimit(4, 254)
        .setRegex(NAME_REGEX);

    registerInputField("passphrase", SAKURA_STRING_TYPE)
        .setComment("Passphrase of the user as base64 encoded string.")
        .setLimit(8, 4096)
        .setRegex(BASE64_REGEX);

    registerInputField("is_admin", SAKURA_BOOL_TYPE)
        .setComment("Set this to 1 to register the new user as admin.")
        .setDefault(false);

    //----------------------------------------------------------------------------------------------
    // output
    //----------------------------------------------------------------------------------------------

    registerOutputField("id", SAKURA_STRING_TYPE).setComment("ID of the new user.");

    registerOutputField("name", SAKURA_STRING_TYPE).setComment("Name of the new user.");

    registerOutputField("is_admin", SAKURA_BOOL_TYPE).setComment("True, if user is an admin.");

    registerOutputField("projects", SAKURA_ARRAY_TYPE)
        .setComment(
            "Json-array with all assigned projects "
            "together with role and project-admin-status.");

    registerOutputField("created_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when resource was created.");

    registerOutputField("created_by", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who created the resource.");

    registerOutputField("updated_at", SAKURA_STRING_TYPE)
        .setComment("Timestamp, when resource was updated.");

    registerOutputField("updated_by", SAKURA_STRING_TYPE)
        .setComment("ID of the user, who updated the resource.");

    //----------------------------------------------------------------------------------------------
    //
    //----------------------------------------------------------------------------------------------
}

/**
 * @brief runTask
 */
bool
CreateUserV1M0::runTask(BlossomIO& blossomIO,
                        const Hanami::UserContext& userContext,
                        BlossomStatus& status,
                        Hanami::ErrorContainer& error)
{
    // check if admin
    if (userContext.isAdmin == false) {
        status.statusCode = UNAUTHORIZED_RTYPE;
        return false;
    }

    const std::string newUserId = blossomIO.input["id"];
    const std::string creatorId = userContext.userId;

    // genreate hash from passphrase and random salt
    std::string pwHash;
    const std::string salt = generateUuid().toString();
    const std::string saltedPw = std::string(blossomIO.input["passphrase"]) + salt;
    Hanami::generate_SHA_256(pwHash, saltedPw);

    // convert values
    UserTable::UserDbEntry dbEntry;
    dbEntry.id = newUserId;
    dbEntry.name = blossomIO.input["name"];
    dbEntry.pwHash = pwHash;
    dbEntry.isAdmin = blossomIO.input["is_admin"];
    dbEntry.salt = salt;

    // add new user to table
    const ReturnStatus ret = UserTable::getInstance()->addUser(dbEntry, userContext, error);
    if (ret == INVALID_INPUT) {
        status.errorMessage = "User with id '" + newUserId + "' already exist.";
        status.statusCode = CONFLICT_RTYPE;
        LOG_DEBUG(status.errorMessage);
        return false;
    }
    if (ret == ERROR) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    // get new created user from database
    if (UserTable::getInstance()->getUser(blossomIO.output, newUserId, false, error) != OK) {
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        return false;
    }

    return true;
}
