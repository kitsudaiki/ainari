/**
 * @file        enums.h
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

#ifndef HANAMI_ENUMS_H
#define HANAMI_ENUMS_H

#include <stdint.h>

enum OutputType {
    PLAIN_OUTPUT = 0,
    BOOL_OUTPUT = 1,
    INT_OUTPUT = 2,
    FLOAT_OUTPUT = 3,
};

enum ClusterProcessingMode : uint8_t {
    NORMAL_MODE = 0,
    TRAIN_FORWARD_MODE = 1,
    TRAIN_BACKWARD_MODE = 2,
};

enum HttpRequestType {
    UNKNOWN_HTTP_TYPE = 0,
    DELETE_TYPE = 1,
    GET_TYPE = 2,
    HEAD_TYPE = 3,
    POST_TYPE = 4,
    PUT_TYPE = 5,
};

enum HttpResponseTypes {
    // 1xx Informational
    CONTINUE_RTYPE = 100,
    SWITCHIN_PROTOCOLS_RTYPE = 101,
    PROCESSING_RTYPE = 102,
    EARLY_HINTS_RTYPE = 103,

    // 2xx Succesful
    OK_RTYPE = 200,
    CREATED_RTYPE = 201,
    ACCEPTED_RTYPE = 202,
    NON_AUTHORITATIVE_INFO_RTYPE = 203,
    NO_CONTENT_RTYPE = 204,
    RESET_CONTENT_RTYPE = 205,
    PARTIAL_CONTENT_RTYPE = 206,
    MULTI_STATUS_RTYPE = 207,
    ALREADY_REPORETED_RTYPE = 208,
    IM_USED_RTYPE = 226,

    // 3xx Redirection
    MULTIPLE_CHOICES_RTYPE = 300,
    MOVED_PERMANENTALLY_RTYPE = 301,
    FOUND_RTYPE = 302,
    SEE_OTHER_RTYPE = 303,
    NOT_MODIFIED_RTYPE = 304,
    USE_PROXY_RTYPE = 305,
    SWITCH_PROXY_RTYPE = 306,
    TEMPORARY_REDIRECT_RTYPE = 307,
    PERMANENT_REDIRECT_RTYPE = 308,

    // 4xx Client Error
    BAD_REQUEST_RTYPE = 400,
    UNAUTHORIZED_RTYPE = 401,
    PAYMENT_REQUIRED_RTYPE = 402,
    FORBIDDEN_RTYPE = 403,
    NOT_FOUND_RTYPE = 404,
    MOTHOD_NOT_ALLOWED_RTYPE = 405,
    NOT_ACCEPTABLE_RTYPE = 406,
    PROXY_AUTHENTICATION_REQUIRED_RTYPE = 407,
    REQUEST_TIMEOUT_RTYPE = 408,
    CONFLICT_RTYPE = 409,
    GONE_RTYPE = 410,
    LENGTH_REQUIRED_RTYPE = 411,
    PRECONDITION_FAILED_RTYPE = 412,
    PAYLOAD_TOO_LARGE_RTYPE = 413,
    URI_TOO_LONG_RTYPE = 414,
    UNSUPPORTED_MEDIA_TYPE_RTYPE = 415,
    RANGE_NOT_SATISFIEABLE_RTYPE = 416,
    EXPECTEATION_FAILED_RTYPE = 417,
    I_AM_NOT_A_TEAPOT_RTYPE = 418,
    MISDIRECTED_REQUEST_RTYPE = 421,
    UNPROCESSABLE_ENTITY_RTYPE = 422,
    LOCKED_RTYPE = 423,
    FAILED_REPENDENCY_RTYPE = 424,
    TOO_EARLY_RTYPE = 425,
    UPGRADE_REQUIRED_RTYPE = 426,
    PRECONDTION_REQUIRED_RESPONSE = 428,
    TOO_MANY_REQUESTES_RTYPE = 429,
    REQUEST_HEADER_FIELDS_TOO_LARGE_RTYPE = 431,
    UNAVAILABLE_FOR_LEGAL_REASONS_RTYPE = 451,

    // 5xx Server Error
    INTERNAL_SERVER_ERROR_RTYPE = 500,
    NOT_IMPLEMENTED_RTYPE = 501,
    BAD_GATEWAY_RTYPE = 502,
    SERVICE_UNAVAILABLE_RTYPE = 503,
    GATEWAY_TIMEOUT_RTYPE = 504,
    HTTP_VERSION_NOT_SUPPORTED_RTYPE = 505,
    VARIANT_ALSO_NEGOTIATES_RTYPE = 506,
    INSUFFICIENT_STORAGE_RTYPE = 507,
    LOOP_DETECTED_RTYPE = 508,
    NOT_EXTENDED_RTYPE = 510,
    NETWORK_AUTHENTICATION_REQUIRED_RTYPE = 511
};

#endif  // HANAMI_ENUMS_H
