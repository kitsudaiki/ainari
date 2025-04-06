/**
 * @file        power_measuring.h
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

#ifndef HANAMI_POWER_MEASURING_H
#define HANAMI_POWER_MEASURING_H

#include <hanami_common/logger.h>
#include <hanami_common/threading/thread.h>
#include <hanami_hardware/value_container.h>

#include <mutex>

namespace Hanami
{
struct RequestMessage;
}

class ValueContainer;

class PowerMeasuring : public Hanami::Thread
{
   public:
    static PowerMeasuring* getInstance()
    {
        if (instance == nullptr) {
            instance = new PowerMeasuring();
        }
        return instance;
    }
    ~PowerMeasuring();

    json getJson();

   protected:
    void run();

   private:
    PowerMeasuring();
    static PowerMeasuring* instance;

    ValueContainer m_valueContainer;
};

#endif  // HANAMI_POWER_MEASURING_H
