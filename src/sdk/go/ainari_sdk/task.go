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

func CreateSnapshotSaveTask(context AccessContext, toriiPort int, name, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/snapshot_save", virtual_machineUuid)
	jsonBody := map[string]interface{}{
		"name": name,
	}
	return SendPost(context, address, path, jsonBody)
}

// CreateSnapshotRestoreTask resets the root-disk of a virtual machine to an image, which must be a
// snapshot.
func CreateSnapshotRestoreTask(context AccessContext, toriiPort int, virtual_machineUuid, imageUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/snapshot_restore", virtual_machineUuid)
	jsonBody := map[string]interface{}{
		"image_uuid": imageUuid,
	}
	return SendPost(context, address, path, jsonBody)
}

// GetTask reads a single task. The tasks are not bound to a virtual machine anymore, but the
// torii-port still selects the sakura-host, which holds the task.
func GetTask(context AccessContext, toriiPort int, taskUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/task/%s", taskUuid)
	vars := map[string]interface{}{}
	return SendGet(context, address, path, vars)
}

// ListTask lists all tasks of the sakura-host behind the given torii-port.
func ListTask(context AccessContext, toriiPort int) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := "v1alpha/task"
	vars := map[string]interface{}{}
	return SendGet(context, address, path, vars)
}

func AbortTask(context AccessContext, toriiPort int, taskUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/task/%s/abort", taskUuid)
	vars := map[string]interface{}{}
	return SendPut(context, address, path, vars)
}
