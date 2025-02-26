/**
 * @file        auth_check.cpp
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

#include "auth_check.h"

#include <hanami_common/functions/string_functions.h>
#include <hanami_policies/policy.h>
#include <hanami_root.h>
#include <jwt-cpp/jwt.h>

/**
 * @brief check if token is valid
 *
 * @param result resulting data from the parsed token
 * @param token token to check
 * @param endpoint endpoint to check against the policies
 * @param httpType http-type to check for policies
 * @param errorMessage reference for error-output
 *
 * @return false, if token-check failed, else true
 */
bool
validateToken(json& result,
              const std::string& token,
              const std::string& endpoint,
              const HttpRequestType httpType,
              std::string& errorMessage)
{
    try {
        auto decodedToken = jwt::decode(token);
        auto verifier = jwt::verify().allow_algorithm(
            jwt::algorithm::hs256{(const char*)HanamiRoot::tokenKey.data()});

        verifier.verify(decodedToken);

        // copy data of token into the output
        for (const auto& payload : decodedToken.get_payload_json()) {
            try {
                result = json::parse(payload.second.to_str());
            }
            catch (const json::parse_error& ex) {
                errorMessage = "Error while parsing decoded token: " + std::string(ex.what());
                return false;
            }
        }
    }
    catch (const std::exception& ex) {
        errorMessage = "Failed to validate JWT-Token with error: " + std::string(ex.what());
        return false;
    }

    if (endpoint != "") {
        if (result.contains("role") == false) {
            errorMessage = "Role is missing in token";
            return false;
        }
        // check policy
        const std::string role = result["role"];
        if (Hanami::Policy::getInstance()->checkUserAgainstPolicy(endpoint, httpType, role)
            == false)
        {
            errorMessage = "Access denied by policy";
            return false;
        }
    }

    // remove irrelevant fields
    result.erase("pw_hash");
    result.erase("creator_id");
    result.erase("exp");
    result.erase("iat");
    result.erase("nbf");
    result.erase("created_at");

    return true;
}
