/**
 * @file        temperature_measuring.h
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

#ifndef HANAMI_TEMPERATURE_MEASURING_H
#define HANAMI_TEMPERATURE_MEASURING_H

#include <hanami_common/logger.h>
#include <hanami_common/threading/thread.h>
#include <hanami_hardware/value_container.h>

namespace Hanami
{
struct RequestMessage;
}

class TemperatureMeasuring : public Hanami::Thread
{
   public:
    static TemperatureMeasuring* getInstance()
    {
        if (instance == nullptr) {
            instance = new TemperatureMeasuring();
        }
        return instance;
    }
    ~TemperatureMeasuring();

    json getJson();

   protected:
    void run();

   private:
    TemperatureMeasuring();
    static TemperatureMeasuring* instance;

    ValueContainer m_valueContainer;
};

#endif  // HANAMI_TEMPERATURE_MEASURING_H
