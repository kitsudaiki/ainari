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

func CreateCheckpointSaveTask(context AccessContext, toriiPort int, name, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/checkpoint_save", virtual_machineUuid)
	jsonBody := map[string]interface{}{
		"name": name,
	}
	return SendPost(context, address, path, jsonBody)
}

func CreateCheckpointRestoreTask(context AccessContext, toriiPort int, name, virtual_machineUuid, checkpointUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/checkpoint_restore", virtual_machineUuid)
	jsonBody := map[string]interface{}{
		"name":            name,
		"checkpoint_uuid": checkpointUuid,
	}
	return SendPost(context, address, path, jsonBody)
}

func GetTask(context AccessContext, toriiPort int, taskUuid, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/task/%s", virtual_machineUuid, taskUuid)
	vars := map[string]interface{}{}
	return SendGet(context, address, path, vars)
}

func ListTask(context AccessContext, toriiPort int, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/task", virtual_machineUuid)
	vars := map[string]interface{}{}
	return SendGet(context, address, path, vars)
}

func AbortTask(context AccessContext, toriiPort int, taskUuid, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/task/%s/abort", virtual_machineUuid, taskUuid)
	vars := map[string]interface{}{}
	return SendPut(context, address, path, vars)
}
