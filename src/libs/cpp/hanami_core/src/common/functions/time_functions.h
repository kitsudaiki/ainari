/**
 * @file        time.h
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

#ifndef HANAMI_TIME_H
#define HANAMI_TIME_H

#include <chrono>
#include <iomanip>
#include <sstream>
#include <string>

typedef std::chrono::milliseconds chronoMilliSec;
typedef std::chrono::microseconds chronoMicroSec;
typedef std::chrono::nanoseconds chronoNanoSec;
typedef std::chrono::seconds chronoSec;
typedef std::chrono::high_resolution_clock::time_point chronoTimePoint;
typedef std::chrono::high_resolution_clock chronoClock;

namespace Hanami
{

/**
 * @brief get the current datetime of the system
 *
 * @return datetime as string
 */
inline const std::string
getDatetime()
{
    std::time_t now = std::time(nullptr);

    // convert to UTC time
    std::tm* gmtime = std::gmtime(&now);

    // convert to string
    std::ostringstream oss;
    oss << std::put_time(gmtime, "%Y-%m-%d %H:%M:%S");

    return oss.str();
}

/**
 * @brief convert chrono-timestamp into a string in UTC time
 *
 * @param time chrono-timeshamp, which should be converted
 * @param format format to convert into
 *
 * @return string with the converted timestamp
 */
inline const std::string
serializeTimePoint(const std::chrono::high_resolution_clock::time_point& time,
                   const std::string& format = "%Y-%m-%d %H:%M:%S")
{
    std::time_t tt = std::chrono::system_clock::to_time_t(time);
    std::tm tm = *std::gmtime(&tt);
    std::stringstream ss;
    ss << std::put_time(&tm, format.c_str());
    return ss.str();
}

}  // namespace Hanami

#endif  // HANAMI_TIME_H
