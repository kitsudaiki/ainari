/**
 * @file       progress_bar.cpp
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

#include <hanami_common/progress_bar.h>

namespace Hanami
{

/**
 * @brief constructor
 *
 * @param maxBarWidth maximum number of character in weidth for displaying the progress-bar
 */
ProgressBar::ProgressBar(const uint32_t maxBarWidth) : m_maxBarWidth(maxBarWidth) {}

/**
 * @brief update the output of the progress-bar with a new progress-state
 *
 * @param newProgress new progress-value to display
 *
 * @return true, is 100% or more as input, else false
 */
bool
ProgressBar::updateProgress(const float newProgress)
{
    bool result = false;
    m_progress = newProgress;

    // check if 1.0 or more are reached and update result for finish
    if (m_progress >= 1.0f) {
        m_progress = 1.0f;
        result = true;
    }

    // print progress
    std::cout << "[";
    const uint32_t pos = m_maxBarWidth * m_progress;
    for (uint32_t i = 0; i < m_maxBarWidth; i++) {
        if (i < pos) {
            std::cout << "=";
        }
        else if (i == pos) {
            std::cout << ">";
        }
        else {
            std::cout << " ";
        }
    }
    std::cout << "] " << uint32_t(m_progress * 100.0) << " %\r";
    std::cout.flush();

    // in case that the progress is complete, then finish the line to avoid that it disappears
    if (m_progress >= 1.0f) {
        std::cout << std::endl;
    }

    return result;
}

}  // namespace Hanami
