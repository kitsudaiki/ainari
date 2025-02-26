/**
 * @file       symmetric_encryption_test.cpp
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

#include "symmetric_encryption_test.h"

#include <hanami_crypto/symmetric_encryption.h>

namespace Hanami
{

Symmetric_Encryption_Test::Symmetric_Encryption_Test()
    : Hanami::CompareTestHelper("Symmetric_Encryption_Test")
{
    encrypt_decrypt_AES_256();
}

/**
 * @brief encrypt_decrypt_AES_256
 */
void
Symmetric_Encryption_Test::encrypt_decrypt_AES_256()
{
    Hanami::ErrorContainer error;

    const std::string testData = "this is a test-string";
    CryptoPP::SecByteBlock key((unsigned char*)"asdf", 4);
    std::string encryptionResult = "";
    std::string decryptionResult = "";

    TEST_EQUAL(encrypt_AES_256_CBC(encryptionResult, testData, key, error), true);
    TEST_EQUAL(decrypt_AES_256_CBC(decryptionResult, encryptionResult, key, error), true);

    TEST_NOT_EQUAL(encryptionResult, testData);
    TEST_EQUAL(decryptionResult, testData);
}

}  // namespace Hanami
