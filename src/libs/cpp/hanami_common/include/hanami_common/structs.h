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

#include "defines.h"
#include "enums.h"

#define UNINTI_POINT_32 0x0FFFFFFF

class Cluster;

namespace Hanami
{

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

struct RequestMessage {
    HttpRequestType httpType = GET_TYPE;
    std::string targetEndpoint = "";
    std::string inputValues = "{}";
};

//==================================================================================================

struct UserContext {
    std::string userId = "";
    std::string projectId = "";
    bool isAdmin = false;
    bool isProjectAdmin = false;
    std::string token = "";
};

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

struct WorkerTask {
    Cluster* cluster = nullptr;
    uint32_t hexagonId = UNINIT_STATE_32;
    uint32_t blockId = UNINIT_STATE_32;
    ClusterProcessingMode mode = ClusterProcessingMode::TRAIN_BACKWARD_MODE;
    uint8_t padding[7];
};
static_assert(sizeof(WorkerTask) == 24);

//==================================================================================================

}  // namespace Hanami

#endif  // HANAMI_STRUCTS_H
