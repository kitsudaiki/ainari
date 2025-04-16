/**
 * @file        structs.h
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

#ifndef HANAMI_STRUCTS_H
#define HANAMI_STRUCTS_H

#include <stdint.h>

#include <cstring>
#include <string>
#include <vector>

#define UNINTI_POINT_32 0x0FFFFFFF

//==================================================================================================

struct NameEntry {
    char name[255];
    uint8_t nameSize = 0;

    NameEntry() { memset(name, 0, 255); }

    const std::string getName() const
    {
        // precheck
        if (nameSize == 0 || nameSize > 254) {
            return std::string("");
        }

        return std::string(name, nameSize);
    }

    bool setName(const std::string& newName)
    {
        // precheck
        if (newName.size() > 254 || newName.size() == 0) {
            return false;
        }

        // copy string into char-buffer and set explicit the escape symbol to be absolut sure
        // that it is set to absolut avoid buffer-overflows
        strncpy(name, newName.c_str(), newName.size());
        name[newName.size()] = '\0';
        nameSize = newName.size();

        return true;
    }

    bool operator==(NameEntry& rhs)
    {
        if (nameSize != rhs.nameSize) {
            return false;
        }
        if (strncmp(name, rhs.name, nameSize) != 0) {
            return false;
        }

        return true;
    }

    bool operator!=(NameEntry& rhs) { return (*this == rhs) == false; }
};
static_assert(sizeof(NameEntry) == 256);

//==================================================================================================

struct Position {
    uint32_t x = UNINTI_POINT_32;
    uint32_t y = UNINTI_POINT_32;
    uint32_t z = UNINTI_POINT_32;
    uint32_t w = UNINTI_POINT_32;

    Position() {}

    Position(const Position& other)
    {
        x = other.x;
        y = other.y;
        z = other.z;
    }

    Position(const uint32_t x, const uint32_t y, const uint32_t z)
    {
        this->x = x;
        this->y = y;
        this->z = z;
    }

    Position& operator=(const Position& other)
    {
        if (this != &other) {
            x = other.x;
            y = other.y;
            z = other.z;
        }

        return *this;
    }

    bool operator==(const Position& other) const
    {
        return (this->x == other.x && this->y == other.y && this->z == other.z);
    }

    bool operator!=(const Position& other) const
    {
        return (this->x != other.x || this->y != other.y || this->z != other.z);
    }

    bool isValid() const
    {
        return (x != UNINTI_POINT_32 && y != UNINTI_POINT_32 && z != UNINTI_POINT_32);
    }

    const std::string toString() const
    {
        return "[ " + std::to_string(x) + " , " + std::to_string(y) + " , " + std::to_string(z)
               + " ]";
    }
};

//==================================================================================================

enum ReturnStatus {
    OK = 0,
    INVALID_INPUT = 1,
    ERROR = 2,
};

enum OutputType {
    PLAIN_OUTPUT = 0,
    BOOL_OUTPUT = 1,
    INT_OUTPUT = 2,
    FLOAT_OUTPUT = 3,
};

struct InputMeta {
    std::string name = "";
    uint32_t targetHexagonId = UNINTI_POINT_32;

    InputMeta(const std::string& name, const uint32_t hexagonId)
    {
        this->name = name;
        this->targetHexagonId = hexagonId;
    }
};

struct OutputMeta {
    std::string name = "";
    uint32_t targetHexagonId = UNINTI_POINT_32;
    OutputType type = PLAIN_OUTPUT;

    OutputMeta(const std::string& name, const uint32_t hexagonId, const OutputType outputType)
    {
        this->name = name;
        this->targetHexagonId = hexagonId;
        this->type = outputType;
    }
};

struct AxonMeta {
    uint32_t sourceId = UNINTI_POINT_32;
    uint32_t targetId = UNINTI_POINT_32;

    AxonMeta(const uint32_t sourceId, const uint32_t targetId)
    {
        this->sourceId = sourceId;
        this->targetId = targetId;
    }
};

struct ClusterMeta {
    uint32_t version = 0;
    float neuronCooldown = 1000000000.f;
    uint32_t refractoryTime = 1;
    uint32_t maxConnectionDistance = 1;

    std::vector<Position> hexagons;
    std::vector<AxonMeta> axons;
    std::vector<InputMeta> inputs;
    std::vector<OutputMeta> outputs;

    ClusterMeta() {}

    void setSettings(const float neuronCooldown,
                     const uint32_t refractoryTime,
                     const uint32_t maxConnectionDistance)
    {
        this->neuronCooldown = neuronCooldown;
        this->refractoryTime = refractoryTime;
        this->maxConnectionDistance = maxConnectionDistance;
    }

    uint32_t addHexagon(const uint32_t x, const uint32_t y, const uint32_t z)
    {
        hexagons.push_back(Position(x, y, z));
        return hexagons.size() - 1;
    }

    void addAxon(const uint32_t sourceId, const uint32_t targetId)
    {
        axons.push_back(AxonMeta(sourceId, targetId));
    }

    void addInput(const std::string& name, const uint32_t hexagonId)
    {
        inputs.push_back(InputMeta(name, hexagonId));
    }

    void addOutput(const std::string& name, const uint32_t hexagonId, const uint8_t outputType)
    {
        outputs.push_back(OutputMeta(name, hexagonId, static_cast<OutputType>(outputType)));
    }
};

#endif  // HANAMI_STRUCTS_H
