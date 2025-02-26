/**
 * @file       common_test.cpp
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

#include "common_test.h"

#include <hanami_common/buffer/data_buffer.h>
#include <hanami_crypto/common.h>

namespace Hanami
{

Common_Test::Common_Test() : Hanami::CompareTestHelper("Common_Test")
{
    hexEncode_test();

    encodeBase64_test();
    decodeBase64_test();

    base64ToBase64Url_test();
    base64UrlToBase64_test();
}

/**
 * hexEncode_test
 */
void
Common_Test::hexEncode_test()
{
    const std::string input = "qwertzuiop";
    std::string result;

    hexEncode(result, input.c_str(), input.size());

    TEST_EQUAL(result, "71776572747A75696F70");
}

/**
 * encodeBase64_test
 */
void
Common_Test::encodeBase64_test()
{
    std::string output = "";
    std::string input = "";

    output = "";
    input = "asdfasdfasdf123a";
    encodeBase64(output, input.c_str(), input.size());
    TEST_EQUAL(output, "YXNkZmFzZGZhc2RmMTIzYQ==");

    output = "";
    input = "3256zu";
    encodeBase64(output, input.c_str(), input.size());
    TEST_EQUAL(output, "MzI1Nnp1");

    output = "";
    input = "5i";
    encodeBase64(output, input.c_str(), input.size());
    TEST_EQUAL(output, "NWk=");
}

/**
 * decodeBase64_test
 */
void
Common_Test::decodeBase64_test()
{
    std::string decode = "";
    std::string result;

    decodeBase64(result, "YXNkZmFzZGZhc2RmMTIzYQ==");
    TEST_EQUAL(result, "asdfasdfasdf123a");

    decodeBase64(result, "MzI1Nnp1");
    TEST_EQUAL(result, "3256zu");

    decodeBase64(result, "NWk=");
    TEST_EQUAL(result, "5i");
}

/**
 * base64ToBase64Url_test
 */
void
Common_Test::base64ToBase64Url_test()
{
    std::string input = "ab/4+3==";

    TEST_EQUAL(base64ToBase64Url(input), true);

    TEST_EQUAL(input, "ab_4-3");
}

/**
 * base64UrlToBase64_test
 */
void
Common_Test::base64UrlToBase64_test()
{
    std::string input = "ab_4-3";

    TEST_EQUAL(base64UrlToBase64(input), true);

    TEST_EQUAL(input, "ab/4+3==");
}

}  // namespace Hanami
