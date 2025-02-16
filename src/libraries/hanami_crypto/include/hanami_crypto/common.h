/**
 * @file       common.h
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

#ifndef CRYPTO_COMMON_H
#define CRYPTO_COMMON_H

#include <stdint.h>

#include <string>

namespace Hanami
{
struct DataBuffer;

//--------------------------------------------------------------------------------------------------
// HEX
//--------------------------------------------------------------------------------------------------
void hexEncode(std::string& result, const void* data, const uint64_t dataSize);

//--------------------------------------------------------------------------------------------------
// BASE64
//--------------------------------------------------------------------------------------------------
void encodeBase64(std::string& output, const void* data, const uint64_t dataSize);
bool decodeBase64(DataBuffer& result, const std::string& input);
bool decodeBase64(std::string& result, const std::string& input);

bool base64ToBase64Url(std::string& base64);
bool base64UrlToBase64(std::string& base64Url);

}  // namespace Hanami

#endif  // CRYPTO_COMMON_H
