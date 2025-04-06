/**
 * @file        value_container.cpp
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

#include <hanami_hardware/value_container.h>

/**
 * @brief constructor
 */
ValueContainer::ValueContainer()
{
    m_valueSections.push_back(ValueSection(60));
    m_valueSections.push_back(ValueSection(60));
    m_valueSections.push_back(ValueSection(24));
    m_valueSections.push_back(ValueSection(365));
}

/**
 * @brief add new value
 *
 * @param newValue new value to add the the list of seconds
 */
void
ValueContainer::addValue(const float newValue)
{
    addValue(newValue, 0);
}

/**
 * @brief add new value to a specific value-section
 *
 * @param newValue new value to add
 * @param sectionId position of the value-section inside the vector
 */
void
ValueContainer::addValue(const float newValue, const uint64_t sectionId)
{
    // break-condition
    if (sectionId >= m_valueSections.size()) {
        return;
    }

    // set new value
    ValueSection* currentSection = &m_valueSections[sectionId];
    currentSection->values[currentSection->pos] = newValue;
    currentSection->pos++;

    // handle overflow
    if (currentSection->pos >= currentSection->values.size()) {
        currentSection->pos = 0;

        // calc overflow-value
        float valueOverflow = 0.0f;
        for (const float val : currentSection->values) {
            valueOverflow += val;
        }
        if (currentSection->values.size() != 0) {
            valueOverflow /= currentSection->values.size();
        }
        else {
            valueOverflow = 0.0f;
        }

        addValue(valueOverflow, sectionId + 1);
    }
}

/**
 * @brief convert all value-sections to a json-like object
 *
 * @param result data-item with all information
 */
json
ValueContainer::toJson()
{
    json result = json::object();
    result["seconds"] = appendSectionToJson(0);
    result["minutes"] = appendSectionToJson(1);
    result["hours"] = appendSectionToJson(2);
    result["days"] = appendSectionToJson(3);
    return result;
}

/**
 * @brief convert all value of a value-section into a json-like array object
 *
 * @param sectionId id of the value-section, which should be converted
 *
 * @return data-item with all value of the selected value-section
 */
json
ValueContainer::appendSectionToJson(const uint64_t sectionId)
{
    // precheck
    if (sectionId >= m_valueSections.size()) {
        return nullptr;
    }

    // fill value in a array
    json valueList = json::array();
    ValueSection* tempValueSection = &m_valueSections[sectionId];
    uint64_t pos = tempValueSection->pos;
    for (uint64_t i = 0; i < tempValueSection->values.size(); i++) {
        valueList.push_back(tempValueSection->values.at(pos));
        pos = (pos + 1) % tempValueSection->values.size();
    }

    return valueList;
}
