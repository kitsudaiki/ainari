/**
 * @file       speed_test_helper.h
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

#ifndef SPEED_TEST_HELPER_H
#define SPEED_TEST_HELPER_H

#include <hanami_common/items/table_item.h>
#include <math.h>

#include <chrono>
#include <map>
#include <string>
#include <vector>

namespace Hanami
{

typedef std::chrono::milliseconds chronoMilliSec;
typedef std::chrono::microseconds chronoMicroSec;
typedef std::chrono::nanoseconds chronoNanoSec;
typedef std::chrono::seconds chronoSec;
typedef std::chrono::high_resolution_clock::time_point chronoTimePoint;
typedef std::chrono::high_resolution_clock chronoClock;

class SpeedTestHelper
{
   public:
    enum TIMETYPES {
        SECONDS,
        MILLI_SECONDS,
        MICRO_SECONDS,
        NANO_SECONDS,
    };

    struct TimerSlot {
        std::string name = "";
        std::string unitName = "";
        std::vector<double> values;

        std::chrono::high_resolution_clock::time_point start;
        std::chrono::high_resolution_clock::time_point end;

        void startTimer() { start = std::chrono::system_clock::now(); }
        void stopTimer() { end = std::chrono::system_clock::now(); }

        double getDuration(const TIMETYPES type)
        {
            if (type == SECONDS) {
                return std::chrono::duration_cast<chronoSec>(end - start).count();
            }
            if (type == MILLI_SECONDS) {
                return std::chrono::duration_cast<chronoMilliSec>(end - start).count();
            }
            if (type == MICRO_SECONDS) {
                return std::chrono::duration_cast<chronoMicroSec>(end - start).count();
            }
            if (type == NANO_SECONDS) {
                return std::chrono::duration_cast<chronoNanoSec>(end - start).count();
            }

            return 0;
        }
    };

    SpeedTestHelper();

    void addToResult(const TimerSlot timeSlot);
    void printResult();

   private:
    std::map<std::string, TimerSlot> m_timeslots;

    TableItem m_result;
};

}  // namespace Hanami

#endif  // SPEED_TEST_HELPER_H
