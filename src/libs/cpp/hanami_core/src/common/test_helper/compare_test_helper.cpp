/**
 * @file       compare_test_helper.cpp
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

#include <src/common/test_helper/compare_test_helper.h>

#include <iostream>

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param testName name for output to identify the test within the output
 */
CompareTestHelper::CompareTestHelper(const std::string& testName)
{
    std::cout << "------------------------------" << std::endl;
    std::cout << "start " << testName << std::endl << std::endl;
}

/**
 * @brief destructor
 */
CompareTestHelper::~CompareTestHelper()
{
    std::cout << "tests succeeded: " << m_successfulTests << std::endl;
    std::cout << "tests failed: " << m_failedTests << std::endl;
    std::cout << "------------------------------" << std::endl << std::endl;

    if (m_failedTests > 0) {
        exit(1);
    }
}

}  // namespace Hanami
