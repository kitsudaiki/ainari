/**
 *  @file       hashes.h
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

#ifndef HASHES_H
#define HASHES_H

#include <iostream>
#include <vector>

namespace Hanami
{

/**
 * @brief function for generating random-values
 *        coming from this website:
 *            https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering/
 *
 * @param input seed for random value
 *
 * @return random value
 */
inline uint32_t
pcg_hash(uint32_t& input)
{
    const uint32_t state = input * 747796405u + 2891336453u;
    const uint32_t word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    input = (word >> 22u) ^ word;
    return input;
}

//--------------------------------------------------------------------------------------------------
// SHA256
//--------------------------------------------------------------------------------------------------
bool generate_SHA_256(std::string& result, const std::string& input);

bool generate_SHA_256(std::string& result, const void* input, const uint64_t inputSize);

}  // namespace Hanami

#endif  // HASHES_H
