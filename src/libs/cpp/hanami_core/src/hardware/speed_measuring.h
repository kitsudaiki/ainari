/**
 * @file        speed_measuring.h
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

#ifndef HANAMI_SPEED_MEASURING_H
#define HANAMI_SPEED_MEASURING_H

#include <src/common/logger.h>
#include <src/common/threading/thread.h>
#include <src/hardware/value_container.h>

namespace Hanami
{
struct RequestMessage;
}

class SpeedMeasuring : public Hanami::Thread
{
   public:
    static SpeedMeasuring* getInstance()
    {
        if (instance == nullptr) {
            instance = new SpeedMeasuring();
        }
        return instance;
    }
    ~SpeedMeasuring();

    json getJson();

   protected:
    void run();

   private:
    SpeedMeasuring();
    static SpeedMeasuring* instance;

    ValueContainer m_valueContainer;
};

#endif  // HANAMI_SPEED_MEASURING_H
