/**
 * @file       memory_leak_test_helper.h
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

#ifndef MEMORY_LEAK_TEST_HELPER_H
#define MEMORY_LEAK_TEST_HELPER_H

#include <hanami_common/memory_counter.h>

#include <iostream>
#include <string>

void* operator new(size_t size);
void* operator new[](size_t size);
void operator delete(void* ptr) noexcept;
void operator delete[](void* ptr) noexcept;
void operator delete(void* ptr, std::size_t) noexcept;
void operator delete[](void* ptr, std::size_t) noexcept;

namespace Hanami
{
using Hanami::MemoryCounter;

// stupid workaround to ignore the unused-warning for the iostream-include
// because the compiler is confused by the macro and I don't want to include
// this lib in every files, which uses the test
static_assert(sizeof(std::cout) > 0, "Avoid unused warning");

class MemoryLeakTestHelpter
{
#define REINIT_TEST() \
    m_currentAllocations = MemoryCounter::globalMemoryCounter.numberOfActiveAllocations;

#define CHECK_MEMORY()                                                                             \
    if (MemoryCounter::globalMemoryCounter.numberOfActiveAllocations - m_currentAllocations != 0)  \
    {                                                                                              \
        int64_t ndiff                                                                              \
            = MemoryCounter::globalMemoryCounter.numberOfActiveAllocations - m_currentAllocations; \
        std::cout << std::endl;                                                                    \
        std::cout << "Memory-leak detected" << std::endl;                                          \
        std::cout << "   File: " << __FILE__ << std::endl;                                         \
        std::cout << "   Method: " << __PRETTY_FUNCTION__ << std::endl;                            \
        std::cout << "   Line: " << __LINE__ << std::endl;                                         \
        std::cout << "   Number of missing deallocations: " << (ndiff) << std::endl;               \
        std::cout << std::endl;                                                                    \
        m_failedTests++;                                                                           \
    }                                                                                              \
    else {                                                                                         \
        m_successfulTests++;                                                                       \
    }

   public:
    MemoryLeakTestHelpter(const std::string& testName);
    ~MemoryLeakTestHelpter();

   protected:
    int64_t m_currentAllocations = 0;

    uint32_t m_failedTests = 0;
    uint32_t m_successfulTests = 0;
};

}  // namespace Hanami

#endif  // MEMORY_LEAK_TEST_HELPER_H
