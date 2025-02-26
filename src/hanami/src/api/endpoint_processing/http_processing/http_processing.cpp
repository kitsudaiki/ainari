/**
 * @file        http_processing.cpp
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

#include "http_processing.h"

#include <api/endpoint_processing/auth_check.h>
#include <api/endpoint_processing/blossom.h>
#include <api/endpoint_processing/http_processing/response_builds.h>
#include <api/endpoint_processing/http_processing/string_functions.h>
#include <api/endpoint_processing/http_server.h>
#include <api/http/endpoint_init.h>
#include <database/audit_log_table.h>
#include <hanami_common/functions/time_functions.h>
#include <hanami_common/logger.h>
#include <hanami_root.h>
#include <jwt-cpp/jwt.h>
// #include <jwt-cpp/traits/nlohmann-json/defaults.h>

/**
 * @brief process request and build response
 */
bool
HttpProcessing::processRequest(http::request<http::string_body>& httpRequest,
                               http::response<http::dynamic_body>& httpResponse,
                               Hanami::ErrorContainer& error)
{
    // build default-header for response
    httpResponse.version(httpRequest.version());
    httpResponse.keep_alive(false);
    httpResponse.set(http::field::server, "Hanami");
    httpResponse.result(http::status::ok);
    httpResponse.set(http::field::content_type, "text/plain");

    // collect and prepare relevant data
    const http::verb messageType = httpRequest.method();
    std::string path = std::string(httpRequest.target());
    std::string payload = "{}";

    // Request path must be absolute and not contain "..".
    if (checkPath(path) == false) {
        error.addMessage("Path '" + path + "' is not valid");
        httpResponse.result(http::status::bad_request);
        return false;
    }

    // check if http-type is supported
    if (messageType != http::verb::get && messageType != http::verb::post
        && messageType != http::verb::put && messageType != http::verb::delete_)
    {
        httpResponse.result(http::status::bad_request);
        error.addMessage("Invalid request-method '" + std::string(httpRequest.method_string())
                         + "'");
        beast::ostream(httpResponse.body()) << error.toString();
        return false;
    }

    // get payload of message
    if (messageType == http::verb::post || messageType == http::verb::put) {
        payload = httpRequest.body().data();
    }

    // get token from request-header
    std::string token = "";
    if (httpRequest.count("X-Auth-Token") > 0) {
        token = std::string(httpRequest.at("X-Auth-Token"));
    }

    // handle control-messages
    HttpRequestType hType = static_cast<HttpRequestType>(messageType);
    if (processControlRequest(httpResponse, path, token, payload, hType, error) == false) {
        error.addMessage("Failed to process control-request");
        return false;
    }
    return true;

    // handle default, if nothing was found
    error.addMessage("no matching endpoint found for path '" + path + "'");
    genericError_ResponseBuild(httpResponse, HttpResponseTypes::NOT_FOUND_RTYPE, error.toString());

    return false;
}

/**
 * @brief process control request
 *
 * @param uri requested uri
 * @param token given token coming from the http-header
 * @param inputValues json-formated input-values
 * @param httpType type of the http-request
 * @param error reference for error-output
 *
 * @return true, if successful, else false
 */
bool
HttpProcessing::processControlRequest(http::response<http::dynamic_body>& httpResponse,
                                      const std::string& uri,
                                      const std::string& token,
                                      const std::string& inputValues,
                                      const HttpRequestType httpType,
                                      Hanami::ErrorContainer& error)
{
    Hanami::RequestMessage hanamiRequest;
    BlossomStatus status;
    json result = json::object();
    json inputValuesJson;
    std::string userId;

    do {
        // parse uri
        hanamiRequest.httpType = httpType;
        hanamiRequest.inputValues = inputValues;
        if (parseUri(token, hanamiRequest, uri, status) == false) {
            status.statusCode = BAD_REQUEST_RTYPE;
            status.errorMessage = "Failed to pasre uri: " + uri;
            break;
        }

        // parse input-values
        try {
            inputValuesJson = json::parse(hanamiRequest.inputValues);
            inputValuesJson.erase("token");
        }
        catch (const json::parse_error& ex) {
            status.statusCode = BAD_REQUEST_RTYPE;
            status.errorMessage = "Failed to pasre input-values: " + std::string(ex.what());
            break;
        }

        // handle token-request
        if (hanamiRequest.targetEndpoint == tokenEndpointPath
            && hanamiRequest.httpType == POST_TYPE)
        {
            if (triggerBlossom(result,
                               tokenEndpointPath,
                               POST_TYPE,
                               Hanami::UserContext(),
                               inputValuesJson,
                               status,
                               error)
                == false)
            {
                if (status.statusCode == OK_RTYPE) {
                    status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
                    error.addMessage("Mismatch between return-value and status of blossom-output");
                }
                break;
            }
            break;
        }

        // check authentication
        json tokenData = json::object();
        std::string errorMessage = "";
        if (validateToken(tokenData,
                          token,
                          hanamiRequest.targetEndpoint,
                          hanamiRequest.httpType,
                          errorMessage)
            == false)
        {
            status.statusCode = UNAUTHORIZED_RTYPE;
            status.errorMessage = errorMessage;
            break;
        }

        userId = tokenData["id"];

        // convert http-type to string
        const std::string httpTypeStr = convertType(hanamiRequest.httpType);
        if (hanamiRequest.httpType != GET_TYPE) {
            if (AuditLogTable::getInstance()->addAuditLogEntry(
                    Hanami::getDatetime(), userId, hanamiRequest.targetEndpoint, httpTypeStr, error)
                == false)
            {
                error.addMessage("ERROR: Failed to write audit-log into database");
                status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
                break;
            }
        }

        // make real request
        const Hanami::UserContext userContext = convertContext(tokenData);
        if (triggerBlossom(result,
                           hanamiRequest.targetEndpoint,
                           hanamiRequest.httpType,
                           userContext,
                           inputValuesJson,
                           status,
                           error)
            == false)
        {
            if (status.statusCode == OK_RTYPE) {
                status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
                error.addMessage("Mismatch between return-value and status of blossom-output");
            }
            break;
        }

        break;
    }
    while (true);

    // build responses, based on the status-code
    if (status.statusCode != OK_RTYPE) {
        if (status.statusCode == INTERNAL_SERVER_ERROR_RTYPE) {
            return internalError_ResponseBuild(httpResponse, userId, inputValuesJson, error);
        }
        else {
            const HttpResponseTypes type = static_cast<HttpResponseTypes>(status.statusCode);
            return genericError_ResponseBuild(httpResponse, type, status.errorMessage);
        }
    }

    return success_ResponseBuild(httpResponse, result.dump());
}

/**
 * @brief trigger existing blossom
 *
 * @param result map with resulting items
 * @param blossomName id of the blossom to trigger
 * @param blossomGroupName id of the group of the blossom to trigger
 * @param initialValues input-values for the tree
 * @param status reference for status-output
 * @param error reference for error-output
 *
 * @return true, if successfule, else false
 */
bool
HttpProcessing::triggerBlossom(json& result,
                               const std::string& id,
                               const HttpRequestType type,
                               const Hanami::UserContext& userContext,
                               const json& initialValues,
                               BlossomStatus& status,
                               Hanami::ErrorContainer& error)
{
    LOG_DEBUG("trigger blossom");

    // get initial blossom-item
    Blossom* blossom = mapEndpoint(id, type);
    if (blossom == nullptr) {
        status.statusCode = BAD_REQUEST_RTYPE;
        status.errorMessage
            = "No endpoint found for path '" + id + "' and type " + convertType(type);
        error.addMessage(status.errorMessage);
        return false;
    }

    // inialize a new blossom-leaf for processing
    BlossomIO blossomIO;
    blossomIO.endpoint = id;
    blossomIO.requestType = convertType(type);
    blossomIO.input = initialValues;

    // check input to be complete
    std::string errorMessage;
    if (blossom->validateFieldsCompleteness(
            initialValues, *blossom->getInputValidationMap(), FieldDef::INPUT_TYPE, errorMessage)
        == false)
    {
        status.statusCode = BAD_REQUEST_RTYPE;
        status.errorMessage = errorMessage;
        LOG_DEBUG(status.errorMessage);
        LOG_DEBUG(
            "check of completeness of input-fields failed"
            "Check of blossom '"
            + id + "' in group '" + convertType(type) + "' failed.");
        return false;
    }

    // process blossom
    if (blossom->growBlossom(blossomIO, userContext, status, error) == false) {
        return false;
    }

    // check output to be complete
    if (blossom->validateFieldsCompleteness(blossomIO.output,
                                            *blossom->getOutputValidationMap(),
                                            FieldDef::OUTPUT_TYPE,
                                            errorMessage)
        == false)
    {
        error.addMessage(errorMessage);
        error.addMessage("check of completeness of output-fields failed");
        error.addMessage("Check of blossom '" + id + "' in group '" + convertType(type)
                         + "' failed.");
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        status.errorMessage = "";
        return false;
    }

    // TODO: override only with the output-values to avoid unnecessary conflicts
    result.clear();
    overrideItems(result, blossomIO.output, ALL);

    return checkStatusCode(blossom, id, convertType(type), status, error);
}

/**
 * @brief check if the given status-code is allowed for the endpoint
 *
 * @param blossom pointer to related blossom
 * @param id name of blossom
 * @param type type of the blossom
 * @param status status to check
 * @param error reference for error-output
 */
bool
HttpProcessing::checkStatusCode(Blossom* blossom,
                                const std::string& id,
                                const std::string& type,
                                BlossomStatus& status,
                                Hanami::ErrorContainer& error)
{
    if (status.statusCode == OK_RTYPE) {
        return true;
    }

    bool found = false;
    for (const uint32_t allowed : blossom->errorCodes) {
        if (allowed == status.statusCode) {
            found = true;
        }
    }

    // if given status-code is unexprected, then override it and clear the message
    // to avoid leaking unwanted information
    if (found == false) {
        error.addMessage("Status-code '" + std::to_string(status.statusCode)
                         + "' is not allowed as output for blossom '" + id + "' in group '" + type
                         + "'");
        status.statusCode = INTERNAL_SERVER_ERROR_RTYPE;
        status.errorMessage = "";
        return false;
    }

    return true;
}

/**
 * @brief convert http-type into string
 *
 * @param type type to convert
 *
 * @return string-result
 */
const std::string
HttpProcessing::convertType(const HttpRequestType type)
{
    std::string httpTypeStr = "GET";
    if (type == DELETE_TYPE) {
        httpTypeStr = "DELETE";
    }
    if (type == GET_TYPE) {
        httpTypeStr = "GET";
    }
    if (type == HEAD_TYPE) {
        httpTypeStr = "HEAD";
    }
    if (type == POST_TYPE) {
        httpTypeStr = "POST";
    }
    if (type == PUT_TYPE) {
        httpTypeStr = "PUT";
    }

    return httpTypeStr;
}

/**
 * @brief map the endpoint to the real target
 *
 * @param id request-id
 * @param type requested http-request-type
 *
 * @return pointer to target
 */
Blossom*
HttpProcessing::mapEndpoint(const std::string& id, const HttpRequestType type)
{
    const auto id_it = endpointRules.find(id);
    if (id_it != endpointRules.end()) {
        auto type_it = id_it->second.find(type);
        if (type_it != id_it->second.end()) {
            return type_it->second;
        }
    }

    return nullptr;
}

/**
 * @brief add new custom-endpoint without the parser
 *
 * @param id identifier for the new entry
 * @param httpType http-type (get, post, put, delete)
 * @param group blossom-group
 * @param newBlossom pointer to endpoint action
 *
 * @return false, if id together with http-type is already registered, else true
 */
bool
HttpProcessing::addEndpoint(const std::string& id,
                            const HttpRequestType& httpType,
                            const std::string& group,
                            Blossom* newBlossom)
{
    newBlossom->tag = group;

    // search for id
    auto id_it = endpointRules.find(id);
    if (id_it != endpointRules.end()) {
        // search for http-type
        if (id_it->second.find(httpType) != id_it->second.end()) {
            return false;
        }

        // add new
        id_it->second.emplace(httpType, newBlossom);
    }
    else {
        // add new
        std::map<HttpRequestType, Blossom*> typeEntry;
        typeEntry.emplace(httpType, newBlossom);
        endpointRules.emplace(id, typeEntry);
    }

    return true;
}

/**
 * @brief override data of a data-map with new incoming information
 *
 * @param original data-map with the original key-values, which should be updates with the
 *                 information of the override-map
 * @param override map with the new incoming information
 * @param type type of override
 */
void
HttpProcessing::overrideItems(json& original, const json& override, OverrideType type)
{
    if (type == ONLY_EXISTING) {
        for (const auto& [name, item] : override.items()) {
            if (original.contains(name)) {
                original[name] = item;
            }
        }
    }
    if (type == ONLY_NON_EXISTING) {
        for (const auto& [name, item] : override.items()) {
            if (original.contains(name) == false) {
                original[name] = item;
            }
        }
    }
    else if (type == ALL) {
        for (const auto& [name, item] : override.items()) {
            original[name] = item;
        }
    }
}
