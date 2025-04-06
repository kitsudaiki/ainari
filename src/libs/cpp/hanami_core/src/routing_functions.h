/**
 * @file        routing_functions.h
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

#ifndef HANAMI_ROUTING_FUNCTIONS_H
#define HANAMI_ROUTING_FUNCTIONS_H

#include <hanami_cluster_parser/cluster_meta.h>
#include <stdint.h>

struct NextSides {
    uint8_t sides[5];
};

/**
 * @brief get neighbor-position for a specific side in the hexagon-grid
 *
 * @param sourcePos base-position
 * @param side side
 *
 * @return position of the object, which is connected to this side
 */
inline Hanami::Position
getNeighborPos(const Hanami::Position sourcePos, const uint8_t side)
{
    Hanami::Position result;

    switch (side) {
        case 0:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x - 1;
            }
            else {
                result.x = sourcePos.x;
            }
            result.y = sourcePos.y - 1;
            result.z = sourcePos.z - 1;
            break;
        }
        case 1:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x;
            }
            else {
                result.x = sourcePos.x + 1;
            }
            result.y = sourcePos.y - 1;
            result.z = sourcePos.z - 1;
            break;
        }
        case 2:
        {
            result.x = sourcePos.x;
            result.y = sourcePos.y;
            result.z = sourcePos.z - 1;
            break;
        }
        case 3:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x;
            }
            else {
                result.x = sourcePos.x + 1;
            }
            result.y = sourcePos.y - 1;
            result.z = sourcePos.z;
            break;
        }
        case 4:
        {
            result.x = sourcePos.x + 1;
            result.y = sourcePos.y;
            result.z = sourcePos.z;
            break;
        }
        case 5:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x;
            }
            else {
                result.x = sourcePos.x + 1;
            }
            result.y = sourcePos.y + 1;
            result.z = sourcePos.z;
            break;
        }
        case 8:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x - 1;
            }
            else {
                result.x = sourcePos.x;
            }
            result.y = sourcePos.y + 1;
            result.z = sourcePos.z;
            break;
        }
        case 7:
        {
            result.x = sourcePos.x - 1;
            result.y = sourcePos.y;
            result.z = sourcePos.z;
            break;
        }
        case 6:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x - 1;
            }
            else {
                result.x = sourcePos.x;
            }
            result.y = sourcePos.y - 1;
            result.z = sourcePos.z;
            break;
        }
        case 9:
        {
            result.x = sourcePos.x;
            result.y = sourcePos.y;
            result.z = sourcePos.z + 1;
            break;
        }
        case 10:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x - 1;
            }
            else {
                result.x = sourcePos.x;
            }
            result.y = sourcePos.y + 1;
            result.z = sourcePos.z + 1;
            break;
        }
        case 11:
        {
            if (sourcePos.y % 2 == 0) {
                result.x = sourcePos.x;
            }
            else {
                result.x = sourcePos.x + 1;
            }
            result.y = sourcePos.y + 1;
            result.z = sourcePos.z + 1;
            break;
        }
        default:
            // this state is never ever allowed. If this is reached, there is something totally
            // broken within the source-code
            assert(false);
    }

    return result;
}

/**
 * @brief get possible next sides based on an incoming side
 *
 * @param side incoming-side
 *
 * @return object with all possible next sides
 */
inline NextSides
getNextSides(const uint8_t side)
{
    NextSides nextSides;

    switch (side) {
        case 0:
        {
            nextSides.sides[0] = 1;
            nextSides.sides[1] = 4;
            nextSides.sides[2] = 11;
            nextSides.sides[3] = 5;
            nextSides.sides[4] = 2;
            break;
        }
        case 1:
        {
            nextSides.sides[0] = 2;
            nextSides.sides[1] = 8;
            nextSides.sides[2] = 10;
            nextSides.sides[3] = 7;
            nextSides.sides[4] = 0;
            break;
        }
        case 2:
        {
            nextSides.sides[0] = 0;
            nextSides.sides[1] = 6;
            nextSides.sides[2] = 9;
            nextSides.sides[3] = 3;
            nextSides.sides[4] = 1;
            break;
        }
        case 3:
        {
            nextSides.sides[0] = 5;
            nextSides.sides[1] = 2;
            nextSides.sides[2] = 8;
            nextSides.sides[3] = 10;
            nextSides.sides[4] = 7;
            break;
        }
        case 4:
        {
            nextSides.sides[0] = 8;
            nextSides.sides[1] = 10;
            nextSides.sides[2] = 7;
            nextSides.sides[3] = 0;
            nextSides.sides[4] = 6;
            break;
        }
        case 5:
        {
            nextSides.sides[0] = 7;
            nextSides.sides[1] = 0;
            nextSides.sides[2] = 6;
            nextSides.sides[3] = 9;
            nextSides.sides[4] = 3;
            break;
        }
        case 8:
        {
            nextSides.sides[0] = 6;
            nextSides.sides[1] = 9;
            nextSides.sides[2] = 3;
            nextSides.sides[3] = 1;
            nextSides.sides[4] = 4;
            break;
        }
        case 7:
        {
            nextSides.sides[0] = 3;
            nextSides.sides[1] = 1;
            nextSides.sides[2] = 4;
            nextSides.sides[3] = 11;
            nextSides.sides[4] = 5;
            break;
        }
        case 6:
        {
            nextSides.sides[0] = 4;
            nextSides.sides[1] = 11;
            nextSides.sides[2] = 5;
            nextSides.sides[3] = 2;
            nextSides.sides[4] = 8;
            break;
        }
        case 9:
        {
            nextSides.sides[0] = 11;
            nextSides.sides[1] = 5;
            nextSides.sides[2] = 2;
            nextSides.sides[3] = 8;
            nextSides.sides[4] = 10;
            break;
        }
        case 10:
        {
            nextSides.sides[0] = 9;
            nextSides.sides[1] = 3;
            nextSides.sides[2] = 1;
            nextSides.sides[3] = 4;
            nextSides.sides[4] = 11;
            break;
        }
        case 11:
        {
            nextSides.sides[0] = 10;
            nextSides.sides[1] = 7;
            nextSides.sides[2] = 0;
            nextSides.sides[3] = 6;
            nextSides.sides[4] = 9;
            break;
        }
        default:
            // this state is never ever allowed. If this is reached, there is something totally
            // broken within the source-code
            assert(false);
    }

    return nextSides;
}

#endif  // HANAMI_ROUTING_FUNCTIONS_H
