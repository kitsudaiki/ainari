/**
 * @file        value_container.h
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

#ifndef HANAMI_VALUECONTAINER_H
#define HANAMI_VALUECONTAINER_H

#include <stdint.h>

#include <nlohmann/json.hpp>
#include <vector>

using json = nlohmann::json;

class ValueContainer
{
   public:
    ValueContainer();

    void addValue(const float newValue);
    json toJson();

   private:
    struct ValueSection {
        std::vector<float> values;
        uint64_t pos = 0;

        ValueSection(const uint64_t numberOfValues)
        {
            values = std::vector<float>(numberOfValues, 0.0f);
        }
    };

    std::vector<ValueSection> m_valueSections;

    void addValue(const float newValue, const uint64_t sectionId);
    json appendSectionToJson(const uint64_t sectionId);
};

#endif  // HANAMI_VALUECONTAINER_H
