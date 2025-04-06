/**
 * @file      thread_test.cpp
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

#include "thread_test.h"

#include <hanami_common/threading/event.h>
#include <hanami_common/threading/thread.h>

#include "bogus_event.h"
#include "bogus_thread.h"

namespace Hanami
{

Thread_Test::Thread_Test() : Hanami::MemoryLeakTestHelpter("DataBuffer_Test")
{
    // The first created thread initialize a static instance of a central thread-handler to
    // track all threads. This will not be deleted anytime, so one thread has to be created
    // outside of the test-case.
    Hanami::Thread* testThread = new BogusThread();
    delete testThread;

    create_delete_test();
    create_delete_with_events_test();
}

/**
 * @brief create_delete_test
 */
void
Thread_Test::create_delete_test()
{
    REINIT_TEST();

    Hanami::Thread* testThread = new BogusThread();
    testThread->startThread();
    usleep(100000);
    delete testThread;

    CHECK_MEMORY();
}

/**
 * @brief create_delete_with_events_test
 */
void
Thread_Test::create_delete_with_events_test()
{
    REINIT_TEST();

    Hanami::Thread* testThread = new BogusThread();
    testThread->startThread();
    Event* testEvent = new BogusEvent();
    testThread->addEventToQueue(testEvent);
    usleep(100000);
    delete testThread;

    CHECK_MEMORY();
}

}  // namespace Hanami
