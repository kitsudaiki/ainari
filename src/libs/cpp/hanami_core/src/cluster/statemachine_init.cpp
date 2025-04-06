/**
 * @file        statemachine_init.cpp
 *
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
 *
 *      Licensed under the Apache License, Version 2.0 (the "License");
 *      you may not use cluster file except in compliance with the License.
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

#include "statemachine_init.h"

#include <cluster/cluster.h>
#include <cluster/states/cycle_finish_state.h>
#include <cluster/states/request_state.h>
#include <cluster/states/restore_cluster_state.h>
#include <cluster/states/save_cluster_state.h>
#include <cluster/states/task_handle_state.h>
#include <cluster/states/train_forward_state.h>
#include <hanami_common/statemachine.h>

/**
 * @brief initialize all possible states of the statemachine
 *
 * @param sm reference to the statemachine, which should be initialized
 */
void
initStates(Hanami::Statemachine& sm)
{
    sm.createNewState(TASK_STATE, "Task-handling mode");
    sm.createNewState(TRAIN_STATE, "Train-State");
    sm.createNewState(TRAIN_FORWARD_STATE, "Train state: run");
    sm.createNewState(TRAIN_CYCLE_FINISH_STATE, "Train state: finish-cycle");
    sm.createNewState(REQUEST_STATE, "Request-State");
    sm.createNewState(REQUEST_FORWARD_STATE, "Request state: forward-propagation");
    sm.createNewState(REQUEST_CYCLE_FINISH_STATE, "Request state: finish-cycle");
    sm.createNewState(DIRECT_STATE, "Direct mode");
}

/**
 * @brief initialize events, which should be triggered for specific states
 *
 * @param sm reference to the statemachine, which should be initialized
 * @param cluster pointer to the cluster, where the statemachine belongs to
 * @param taskState pointer the the cluster-specific task-handling-state
 */
void
initEvents(Hanami::Statemachine& sm, Cluster* cluster, TaskHandle_State* taskState)
{
    sm.addEventToState(TASK_STATE, taskState);
    sm.addEventToState(TRAIN_FORWARD_STATE, new TrainForward_State(cluster));
    sm.addEventToState(REQUEST_FORWARD_STATE, new Request_State(cluster));
    sm.addEventToState(TRAIN_CYCLE_FINISH_STATE, new CycleFinish_State(cluster));
    sm.addEventToState(REQUEST_CYCLE_FINISH_STATE, new CycleFinish_State(cluster));
}

/**
 * @brief initialize child-states
 *
 * @param sm reference to the statemachine, which should be initialized
 */
void
initChildStates(Hanami::Statemachine& sm)
{
    // child states training
    sm.addChildState(TRAIN_STATE, TRAIN_FORWARD_STATE);
    sm.addChildState(TRAIN_STATE, TRAIN_CYCLE_FINISH_STATE);

    // child states image request
    sm.addChildState(REQUEST_STATE, REQUEST_FORWARD_STATE);
    sm.addChildState(REQUEST_STATE, REQUEST_CYCLE_FINISH_STATE);
}

/**
 * @brief set initial
 *
 * @param sm reference to the statemachine, which should be initialized
 */
void
initInitialChildStates(Hanami::Statemachine& sm)
{
    sm.setInitialChildState(TRAIN_STATE, TRAIN_FORWARD_STATE);
    sm.setInitialChildState(REQUEST_STATE, REQUEST_FORWARD_STATE);
}

/**
 * @brief initialize transitions between states
 *
 * @param sm reference to the statemachine, which should be initialized
 */
void
initTransitions(Hanami::Statemachine& sm)
{
    // transtions train init
    sm.addTransition(TASK_STATE, TRAIN, TRAIN_STATE);

    // transitions request init
    sm.addTransition(TASK_STATE, REQUEST, REQUEST_STATE);

    // trainsition train-internal
    sm.addTransition(TRAIN_FORWARD_STATE, NEXT, TRAIN_CYCLE_FINISH_STATE);
    sm.addTransition(TRAIN_CYCLE_FINISH_STATE, NEXT, TRAIN_FORWARD_STATE);

    // trainsition request-internal
    sm.addTransition(REQUEST_FORWARD_STATE, NEXT, REQUEST_CYCLE_FINISH_STATE);
    sm.addTransition(REQUEST_CYCLE_FINISH_STATE, NEXT, REQUEST_FORWARD_STATE);

    // transition finish back to task-state
    sm.addTransition(TRAIN_STATE, FINISH_TASK, TASK_STATE);
    sm.addTransition(REQUEST_STATE, FINISH_TASK, TASK_STATE);

    // special transition to tigger the task-state again
    sm.addTransition(TASK_STATE, PROCESS_TASK, TASK_STATE);

    // mode-switches
    sm.addTransition(TASK_STATE, SWITCH_TO_DIRECT_MODE, DIRECT_STATE);
    sm.addTransition(DIRECT_STATE, SWITCH_TO_TASK_MODE, TASK_STATE);
}

/**
 * @brief initialize statemachine of the cluster
 *
 * @param sm reference to the statemachine, which should be initialized
 * @param cluster pointer to the cluster, where the statemachine belongs to
 * @param taskState pointer the the cluster-specific task-handling-state
 */
void
initStatemachine(Hanami::Statemachine& sm, Cluster* cluster, TaskHandle_State* taskState)
{
    initStates(sm);
    initEvents(sm, cluster, taskState);
    initChildStates(sm);
    initInitialChildStates(sm);
    initTransitions(sm);

    // set initial state for the state-machine
    sm.setCurrentState(TASK_STATE);
}
