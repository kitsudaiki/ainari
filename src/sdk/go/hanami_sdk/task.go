/**
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

package hanami_sdk

type TaskInput struct {
	HexagonName        string `json:"hexagon_name"`
	DatasetColumnName  string `json:"dataset_column"`
	DatasetUuid        string `json:"dataset_uuid"`
}

type TaskResult struct {
	HexagonName        string `json:"hexagon_name"`
	DatasetColumnName  string `json:"dataset_column"`
}

func CreateTrainTask(address, token, name, clusterUuid string, inputs, outputs []TaskInput, timeLenght int, skipTlsVerification bool) (map[string]interface{}, error) {
    var inputArray []interface{}
    for _, input := range inputs {
        inputArray = append(inputArray, input)
    }

    var outputArray []interface{}
    for _, output := range outputs {
        outputArray = append(outputArray, output)
    }

	path := "v1alpha/task/train"
	jsonBody := map[string]interface{}{
		"name":         name,
		"cluster_uuid": clusterUuid,
		"inputs":       inputArray,
		"outputs":      outputArray,
		"time_length":  timeLenght,
	}
	return SendPost(address, token, path, jsonBody, skipTlsVerification)
}

func CreateRequestTask(address, token, name, clusterUuid string, inputs []TaskInput, results []TaskResult, timeLenght int, skipTlsVerification bool) (map[string]interface{}, error) {
	var inputArray []interface{}
    for _, input := range inputs {
        inputArray = append(inputArray, input)
    }

    var resultArray []interface{}
    for _, result := range results {
        resultArray = append(resultArray, result)
    }

	path := "v1alpha/task/request"
	jsonBody := map[string]interface{}{
		"name":         name,
		"cluster_uuid": clusterUuid,
		"inputs":       inputArray,
		"results":      resultArray,
		"time_length":  timeLenght,
	}
	return SendPost(address, token, path, jsonBody, skipTlsVerification)
}

func GetTask(address, token, taskId, clusterUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/task"
	vars := map[string]interface{}{
		"uuid":         taskId,
		"cluster_uuid": clusterUuid,
	}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func ListTask(address, token, clusterUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/task/all"
	vars := map[string]interface{}{"cluster_uuid": clusterUuid}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func DeleteTask(address, token, taskUuid, clusterUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/task"
	vars := map[string]interface{}{
		"uuid":         taskUuid,
		"cluster_uuid": clusterUuid,
	}
	return SendDelete(address, token, path, vars, skipTlsVerification)
}
