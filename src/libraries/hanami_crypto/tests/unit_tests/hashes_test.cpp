/**
 * @file       hashes_test.cpp
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

#include "hashes_test.h"

#include <hanami_crypto/hashes.h>

namespace Hanami
{

Hashes_Test::Hashes_Test() : Hanami::CompareTestHelper("Hashes_Test") { generate_SHA_256_test(); }

/**
 * @brief generateSha256_test
 */
void
Hashes_Test::generate_SHA_256_test()
{
    std::string testInput = "test";
    std::string result = "";

    generate_SHA_256(result, testInput.c_str(), testInput.size());

    TEST_EQUAL(result, "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
}

}  // namespace Hanami
