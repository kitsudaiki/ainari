/**
 * @file        statemachine_init.h
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

#ifndef HANAMI_STATEMACHINE_INIT_H
#define HANAMI_STATEMACHINE_INIT_H

class TaskHandle_State;
class CycleFinish_State;
class TableInterpolation_State;
class TableTrainBackward_State;
class TableTrainForward_State;
class Request_State;
class ImageTrainBackward_State;
class TrainForward_State;
class Cluster;

namespace Hanami
{
class EventQueue;
class Statemachine;
}  // namespace Hanami

enum ClusterStates {
    TASK_STATE = 0,
    TRAIN_STATE = 1,
    TRAIN_FORWARD_STATE = 2,
    TRAIN_CYCLE_FINISH_STATE = 3,
    REQUEST_STATE = 4,
    REQUEST_FORWARD_STATE = 5,
    REQUEST_CYCLE_FINISH_STATE = 6,
    DIRECT_STATE = 11,
};

enum ClusterTransitions {
    TRAIN = 100,
    REQUEST = 101,
    CHECKPOINT = 102,
    CLUSTER = 105,
    NEXT = 108,
    FINISH_TASK = 109,
    PROCESS_TASK = 110,
    SWITCH_TO_DIRECT_MODE = 111,
    SWITCH_TO_TASK_MODE = 112,
};

void initStatemachine(Hanami::Statemachine& sm, Cluster* cluster, TaskHandle_State* taskState);

#endif  // HANAMI_STATEMACHINE_INIT_H
