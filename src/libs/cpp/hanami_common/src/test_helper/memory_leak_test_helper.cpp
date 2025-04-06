/**
 * @file       memory_leak_test_helper.cpp
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

#include <hanami_common/test_helper/memory_leak_test_helper.h>

#include <iostream>

//==================================================================================================
// Overrides for new and delete
//==================================================================================================

/**
 * Info: I do not override "void operator delete(void* ptr, size_t size)", which is also available
 *       since c++14, because it not every time used, for example std::map use only the variant
 *       without the size-parameter. So I don't have the size-value in each delete, which is the
 *       reason, why I write this additonally in my allocated memory.
 */

void*
operator new(size_t size)
{
    void* ptr = malloc(size);
    Hanami::increaseGlobalMemoryCounter(0);
    return ptr;
}

void*
operator new[](size_t size)
{
    void* ptr = malloc(size);
    Hanami::increaseGlobalMemoryCounter(0);
    return ptr;
}

void
operator delete(void* ptr) noexcept
{
    free(ptr);
    Hanami::decreaseGlobalMemoryCounter(0);
}

void
operator delete[](void* ptr) noexcept
{
    free(ptr);
    Hanami::decreaseGlobalMemoryCounter(0);
}

void
operator delete(void* ptr, std::size_t) noexcept
{
    free(ptr);
    Hanami::decreaseGlobalMemoryCounter(0);
}

void
operator delete[](void* ptr, std::size_t) noexcept
{
    free(ptr);
    Hanami::decreaseGlobalMemoryCounter(0);
}

//==================================================================================================

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param testName name for output to identify the test within the output
 */
MemoryLeakTestHelpter::MemoryLeakTestHelpter(const std::string& testName)
{
    m_currentAllocations = MemoryCounter::globalMemoryCounter.numberOfActiveAllocations;

    std::cout << "------------------------------" << std::endl;
    std::cout << "start " << testName << std::endl << std::endl;
}

/**
 * @brief destructor
 */
MemoryLeakTestHelpter::~MemoryLeakTestHelpter()
{
    std::cout << "tests succeeded: " << m_successfulTests << std::endl;
    std::cout << "tests failed: " << m_failedTests << std::endl;
    std::cout << "------------------------------" << std::endl << std::endl;

    if (m_failedTests > 0) {
        exit(1);
    }
}

}  // namespace Hanami
