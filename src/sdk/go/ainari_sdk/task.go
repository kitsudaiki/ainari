/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>
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

package ainari_sdk

import (
	"fmt"
)

type TaskInput struct {
	HexagonName        string `json:"hexagon"`
	DatasetColumnName  string `json:"dataset_column"`
	DatasetUuid        string `json:"dataset_uuid"`
}

type TaskResult struct {
	HexagonName        string `json:"hexagon"`
	DatasetColumnName  string `json:"dataset_column"`
}

func CreateTrainTask(context AccessContext, toriiPort int, name, clusterUuid string, inputs, outputs []TaskInput, number_of_epochs, timeLenght int) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
    var inputArray []interface{}
    for _, input := range inputs {
        inputArray = append(inputArray, input)
    }

    var outputArray []interface{}
    for _, output := range outputs {
        outputArray = append(outputArray, output)
    }

	path := fmt.Sprintf("v1alpha/cluster/%s/task/train", clusterUuid)
	jsonBody := map[string]interface{}{
		"name":             name,
		"number_of_epochs": number_of_epochs,
		"inputs":           inputArray,
		"outputs":          outputArray,
		"time_length":      timeLenght,
	}
	return SendPost(context, address, path, jsonBody)
}

func CreateRequestTask(context AccessContext, toriiPort int, name, clusterUuid string, inputs []TaskInput, results []TaskResult, timeLenght int) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	var inputArray []interface{}
    for _, input := range inputs {
        inputArray = append(inputArray, input)
    }

    var resultArray []interface{}
    for _, result := range results {
        resultArray = append(resultArray, result)
    }

	path := fmt.Sprintf("v1alpha/cluster/%s/task/request", clusterUuid)
	jsonBody := map[string]interface{}{
		"name":         name,
		"inputs":       inputArray,
		"results":      resultArray,
		"time_length":  timeLenght,
	}
	return SendPost(context, address, path, jsonBody)
}

func CreateCheckpointSaveTask(context AccessContext, toriiPort int, name, clusterUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/cluster/%s/task/checkpoint_save", clusterUuid)
	jsonBody := map[string]interface{}{
		"name": name,
	}
	return SendPost(context, address, path, jsonBody)
}

func CreateCheckpointRestoreTask(context AccessContext, toriiPort int, name, clusterUuid, checkpointUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/cluster/%s/task/checkpoint_restore", clusterUuid)
	jsonBody := map[string]interface{}{
		"name": name,
		"checkpoint_uuid": checkpointUuid,
	}
	return SendPost(context, address, path, jsonBody)
}

func GetTask(context AccessContext, toriiPort int, taskUuid, clusterUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/cluster/%s/task/%s", clusterUuid, taskUuid)
	vars := map[string]interface{}{}
	return SendGet(context, address, path, vars)
}

func ListTask(context AccessContext, toriiPort int, clusterUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/cluster/%s/task", clusterUuid)
	vars := map[string]interface{}{}
	return SendGet(context, address, path, vars)
}

func DeleteTask(context AccessContext, toriiPort int, taskUuid, clusterUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/cluster/%s/task/%s", clusterUuid, taskUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, address, path, vars)
}

func AbortTask(context AccessContext, toriiPort int, taskUuid, clusterUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/cluster/%s/task/%s/abort", clusterUuid, taskUuid)
	vars := map[string]interface{}{}
	return SendPut(context, address, path, vars)
}
