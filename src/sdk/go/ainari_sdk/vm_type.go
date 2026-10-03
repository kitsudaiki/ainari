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

// CreateVmType creates a new vm-type, which defines the number of cores and the amount of memory
// in MiB of a virtual machine. Only admins are allowed to do this.
func CreateVmType(context AccessContext, name string, numberOfCores int32, amountOfMemory int64) (map[string]interface{}, error) {
	path := "v1alpha/vm_type/admin"
	jsonBody := map[string]interface{}{
		"name":             name,
		"number_of_cores":  numberOfCores,
		"amount_of_memory": amountOfMemory,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

// UpdateVmType updates the values of a vm-type. Only the values, which are not nil, are changed.
// Only admins are allowed to do this.
func UpdateVmType(context AccessContext, vmTypeUuid string, name *string, numberOfCores *int32, amountOfMemory *int64) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/vm_type/%s/admin", vmTypeUuid)
	jsonBody := map[string]interface{}{}
	if name != nil {
		jsonBody["name"] = *name
	}
	if numberOfCores != nil {
		jsonBody["number_of_cores"] = *numberOfCores
	}
	if amountOfMemory != nil {
		jsonBody["amount_of_memory"] = *amountOfMemory
	}
	return SendPut(context, context.HanamiAddress, path, jsonBody)
}

func GetVmType(context AccessContext, vmTypeUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/vm_type/%s", vmTypeUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListVmType(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/vm_type"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

// DeleteVmType deletes a vm-type. Only admins are allowed to do this.
func DeleteVmType(context AccessContext, vmTypeUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/vm_type/%s/admin", vmTypeUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
