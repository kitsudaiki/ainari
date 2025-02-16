/**
 * @file       hashes.cpp
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

#include <cryptopp/filters.h>
#include <cryptopp/modes.h>
#include <cryptopp/sha.h>
#include <hanami_common/functions/string_functions.h>
#include <hanami_crypto/common.h>
#include <hanami_crypto/hashes.h>

namespace Hanami
{

//--------------------------------------------------------------------------------------------------
// SHA256
//--------------------------------------------------------------------------------------------------

/**
 * @brief generate SHA256 hash
 *
 * @param result reference for the resulting string
 * @param input input-string to hash
 *
 * @return false, if input is empty-string, else true
 */
bool
generate_SHA_256(std::string& result, const std::string& input)
{
    return generate_SHA_256(result, &input[0], input.size());
}

/**
 * @brief generate SHA256 hash
 *
 * @param result reference for the resulting string
 * @param input pointer to the input-data
 * @param inputSize size of input-data in number of bytes
 *
 * @return false, if input is invalid, else true
 */
bool
generate_SHA_256(std::string& result, const void* input, const uint64_t inputSize)
{
    if (input == nullptr || inputSize == 0) {
        return false;
    }

    // input MUST be cleared to avoid conflicts
    result.clear();

    CryptoPP::SHA256 hash;
    CryptoPP::byte digest[CryptoPP::SHA256::DIGESTSIZE];

    hash.CalculateDigest(digest, (CryptoPP::byte*)input, inputSize);
    hexEncode(result, digest, sizeof(digest));
    Hanami::toLowerCase(result);

    return true;
}

}  // namespace Hanami
