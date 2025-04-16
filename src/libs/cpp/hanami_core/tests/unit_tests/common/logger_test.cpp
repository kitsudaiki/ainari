/**
 * @file    logger_test.cpp
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

#include "logger_test.h"

#include <src/common/logger.h>

namespace Hanami
{

Logger_Test::Logger_Test() : Hanami::CompareTestHelper("Logger_Test") { logger_test(); }

/**
 * @brief logger_test
 */
void
Logger_Test::logger_test()
{
    // init logger
    bool ret = initConsoleLogger(true);
    TEST_EQUAL(ret, true);

    // create error-container
    ErrorContainer error1;
    error1.addMessage("error1.1");
    error1.addMessage("error1.2");
    error1.addMessage("error1.3");
    error1.addSolution("do nothing1");
    error1.addSolution("do nothing2");
    ErrorContainer error2;
    error2.addMessage("error2");
    error2.addSolution("really nothing");
    ErrorContainer error3;
    error3.addMessage("error3");
    error3.addSolution("really absolutely nothing");

    // write test-data
    TEST_EQUAL(LOG_ERROR(error1), true);
    TEST_EQUAL(LOG_ERROR(error1), true);
    TEST_EQUAL(LOG_ERROR(error2), true);
    TEST_EQUAL(LOG_ERROR(error3), true);

    TEST_EQUAL(LOG_WARNING("warning1"), true);
    TEST_EQUAL(LOG_WARNING("warning2"), true);
    TEST_EQUAL(LOG_WARNING("warning3"), true);

    TEST_EQUAL(LOG_DEBUG("debug1"), true);
    TEST_EQUAL(LOG_DEBUG("debug2"), true);
    TEST_EQUAL(LOG_DEBUG("debug3"), true);

    TEST_EQUAL(LOG_INFO("info1"), true);
    TEST_EQUAL(LOG_INFO("info2"), true);
    TEST_EQUAL(LOG_INFO("info3"), true);
    TEST_EQUAL(LOG_INFO("green-info", GREEN_COLOR), true);
    TEST_EQUAL(LOG_INFO("red-info", RED_COLOR), true);
    TEST_EQUAL(LOG_INFO("blue-info", BLUE_COLOR), true);
    TEST_EQUAL(LOG_INFO("pink-info", PINK_COLOR), true);
}

/**
 * common usage to delete test-file
 */
void
Logger_Test::deleteFile(const std::string filePath)
{
    std::filesystem::path rootPathObj(filePath);
    if (std::filesystem::exists(rootPathObj)) {
        std::filesystem::remove(rootPathObj);
    }
}

}  // namespace Hanami
