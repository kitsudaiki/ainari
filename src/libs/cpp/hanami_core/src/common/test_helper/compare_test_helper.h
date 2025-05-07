/**
 * @file       compare_test_helper.h
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

#ifndef COMPARE_TEST_HELPER_H
#define COMPARE_TEST_HELPER_H

#include <stdint.h>

#include <iostream>
#include <string>

namespace Hanami
{

// stupid workaround to ignore the unused-warning for the iostream-include
// because the compiler is confused by the macro and I don't want to include
// this lib in every files, which uses the test
static_assert(sizeof(std::cout) > 0, "Avoid unused warning");

class CompareTestHelper
{
#define TEST_EQUAL(IS_VAL, SHOULD_VAL)                                  \
    if (IS_VAL != SHOULD_VAL) {                                         \
        m_failedTests++;                                                \
        std::cout << std::endl;                                         \
        std::cout << "Error in Test" << std::endl;                      \
        std::cout << "   File: " << __FILE__ << std::endl;              \
        std::cout << "   Method: " << __PRETTY_FUNCTION__ << std::endl; \
        std::cout << "   Line: " << __LINE__ << std::endl;              \
        std::cout << "   Variable: " << #IS_VAL << std::endl;           \
        std::cout << "   Expected-Value: " << SHOULD_VAL << std::endl;  \
        std::cout << "   Is-Value: " << IS_VAL << std::endl;            \
        std::cout << std::endl;                                         \
    }                                                                   \
    else {                                                              \
        m_successfulTests++;                                            \
    }

#define TEST_NOT_EQUAL(IS_VAL, SHOULD_NOT_VAL)                                 \
    if (IS_VAL == SHOULD_NOT_VAL) {                                            \
        m_failedTests++;                                                       \
        std::cout << std::endl;                                                \
        std::cout << "Error in Test" << std::endl;                             \
        std::cout << "   File: " << __FILE__ << std::endl;                     \
        std::cout << "   Method: " << __PRETTY_FUNCTION__ << std::endl;        \
        std::cout << "   Line: " << __LINE__ << std::endl;                     \
        std::cout << "   Variable: " << #IS_VAL << std::endl;                  \
        std::cout << "   Expected-NOT-Value: " << SHOULD_NOT_VAL << std::endl; \
        std::cout << "   Is-Value: " << IS_VAL << std::endl;                   \
        std::cout << std::endl;                                                \
    }                                                                          \
    else {                                                                     \
        m_successfulTests++;                                                   \
    }

   public:
    CompareTestHelper(const std::string& testName);
    ~CompareTestHelper();

   protected:
    uint32_t m_successfulTests = 0;
    uint32_t m_failedTests = 0;
};

}  // namespace Hanami

#endif  // COMPARE_TEST_HELPER_H
