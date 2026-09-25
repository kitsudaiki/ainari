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

// ReserveVirtualMachine reserves a new virtual machine on one of the sakura-hosts. The image and
// the public-key are not deployed here, but by the task of CreateVirtualMachine. The memorySize
// is given in MiB and the diskSize in GiB.
func ReserveVirtualMachine(context AccessContext, name string, numberOfCores int32, memorySize int64, diskSize int64, networkUuid string) (map[string]interface{}, error) {
	path := "v1alpha/virtual_machine"
	jsonBody := map[string]interface{}{
		"number_of_cores": numberOfCores,
		"memory_size":     memorySize,
		"disk_size":       diskSize,
		"name":            name,
		"network_uuid":    networkUuid,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

// CreateVirtualMachine creates a task on the sakura-host of a reserved virtual machine, which
// installs the image and the public-key in the virtual machine and boots it.
func CreateVirtualMachine(context AccessContext, toriiPort int, virtual_machineUuid, imageUuid, publicKeyUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s", virtual_machineUuid)
	jsonBody := map[string]interface{}{
		"vm_uuid":         virtual_machineUuid,
		"image_uuid":      imageUuid,
		"public_key_uuid": publicKeyUuid,
	}
	return SendPost(context, address, path, jsonBody)
}

// StartVirtualMachine creates a task on the sakura-host of a virtual machine, which boots the
// stopped virtual machine again.
func StartVirtualMachine(context AccessContext, toriiPort int, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/start", virtual_machineUuid)
	jsonBody := map[string]interface{}{}
	return SendPost(context, address, path, jsonBody)
}

// StopVirtualMachine creates a task on the sakura-host of a virtual machine, which shuts down the
// virtual machine. It keeps all of its resources, so it can be started again later.
func StopVirtualMachine(context AccessContext, toriiPort int, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/stop", virtual_machineUuid)
	jsonBody := map[string]interface{}{}
	return SendPost(context, address, path, jsonBody)
}

// RebootVirtualMachine creates a task on the sakura-host of a virtual machine, which reboots the
// running virtual machine.
func RebootVirtualMachine(context AccessContext, toriiPort int, virtual_machineUuid string) (map[string]interface{}, error) {
	address := fmt.Sprintf("%s:%d", context.ToriiBaseAddress, toriiPort)
	path := fmt.Sprintf("v1alpha/virtual_machine/%s/reboot", virtual_machineUuid)
	jsonBody := map[string]interface{}{}
	return SendPost(context, address, path, jsonBody)
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

// GetVirtualMachineCount returns the number of virtual machines of the project.
func GetVirtualMachineCount(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/virtual_machine/count"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}
