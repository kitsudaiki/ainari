/**
 * @file    statemachine_test.cpp
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

#include "progress_bar_test.h"

#include <hanami_common/progress_bar.h>

namespace Hanami
{

ProgressBar_Test::ProgressBar_Test() : Hanami::CompareTestHelper("ProgressBar_Test")
{
    progress_test();
}

/**
 * progress_test
 */
void
ProgressBar_Test::progress_test()
{
    ProgressBar* progressBar = new ProgressBar(100);

    TEST_EQUAL(progressBar->m_maxBarWidth, 100);
    TEST_EQUAL(progressBar->m_progress, 0.0f);

    TEST_EQUAL(progressBar->updateProgress(0.5f), false);
    TEST_EQUAL(progressBar->m_progress, 0.5f);

    TEST_EQUAL(progressBar->updateProgress(1.0f), true);
    TEST_EQUAL(progressBar->m_progress, 1.0f);

    TEST_EQUAL(progressBar->updateProgress(1.5f), true);
    TEST_EQUAL(progressBar->m_progress, 1.0f);

    delete progressBar;
}

}  // namespace Hanami
