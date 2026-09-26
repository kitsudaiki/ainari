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

// AddFloatingIp creates a new floating ip. If floatingIp is an empty string, a free floating ip is selected.
// If virtualMachineUuid is not an empty string, the new floating ip is directly attached to this virtual machine.
func AddFloatingIp(context AccessContext, name, floatingIp, virtualMachineUuid string) (map[string]interface{}, error) {
	path := "v1alpha/floating_ip"
	jsonBody := map[string]interface{}{
		"name": name,
	}
	if floatingIp != "" {
		jsonBody["floating_ip"] = floatingIp
	}
	if virtualMachineUuid != "" {
		jsonBody["virtual_machine_uuid"] = virtualMachineUuid
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

// AttachFloatingIp attaches a floating ip to a virtual machine.
func AttachFloatingIp(context AccessContext, floatingIpUuid, virtualMachineUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/floating_ip/%s/attach", floatingIpUuid)
	jsonBody := map[string]interface{}{
		"virtual_machine_uuid": virtualMachineUuid,
	}
	return SendPut(context, context.HanamiAddress, path, jsonBody)
}

// DetachFloatingIp detaches a floating ip from its virtual machine, so it can be attached to another one.
func DetachFloatingIp(context AccessContext, floatingIpUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/floating_ip/%s/detach", floatingIpUuid)
	jsonBody := map[string]interface{}{}
	return SendPut(context, context.HanamiAddress, path, jsonBody)
}

func GetFloatingIp(context AccessContext, floatingIpUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/floating_ip/%s", floatingIpUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListFloatingIp(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/floating_ip"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func DeleteFloatingIp(context AccessContext, floatingIpUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/floating_ip/%s", floatingIpUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
