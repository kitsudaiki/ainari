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

func CreateVirtualMachine(context AccessContext, name string, numberOfCores int32, memorySize int64, imageUuid, networkUuid string) (map[string]interface{}, error) {
	path := "v1alpha/virtual_machine"
	jsonBody := map[string]interface{}{
		"number_of_cores": numberOfCores,
		"memory_size":     memorySize,
		"image_uuid":      imageUuid,
		"name":            name,
		"network_uuid":    networkUuid,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

func GetVirtualMachine(context AccessContext, virtual_machineUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/virtual_machine/%s", virtual_machineUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListVirtualMachine(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/virtual_machine"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func DeleteVirtualMachine(context AccessContext, virtual_machineUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/virtual_machine/%s", virtual_machineUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
