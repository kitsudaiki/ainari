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

// networkFilterPath returns the path of the packet filter of one direction ("ingress" or
// "egress") of a virtual machine.
func networkFilterPath(virtualMachineUuid, direction string) string {
	return fmt.Sprintf("v1alpha/network_filter/%s/%s", virtualMachineUuid, direction)
}

// AddNetworkFilterIpRanges adds ip-ranges to the packet filter of one direction of a virtual
// machine. Each range is a single address, a subnet in CIDR notation or an explicit range
// "first-last".
func AddNetworkFilterIpRanges(context AccessContext, virtualMachineUuid, direction string, ranges []string) (map[string]interface{}, error) {
	path := networkFilterPath(virtualMachineUuid, direction) + "/ip_range"
	jsonBody := map[string]interface{}{
		"ranges": ranges,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

// DeleteNetworkFilterIpRanges removes ip-ranges from the packet filter of one direction of a
// virtual machine.
func DeleteNetworkFilterIpRanges(context AccessContext, virtualMachineUuid, direction string, ranges []string) (map[string]interface{}, error) {
	path := networkFilterPath(virtualMachineUuid, direction) + "/ip_range"
	jsonBody := map[string]interface{}{
		"ranges": ranges,
	}
	return SendDeleteWithBody(context, context.HanamiAddress, path, jsonBody)
}

// AddNetworkFilterPorts adds ports to the packet filter of one direction of a virtual machine.
// Each entry is a single port or an explicit range "first-last".
func AddNetworkFilterPorts(context AccessContext, virtualMachineUuid, direction string, ports []string) (map[string]interface{}, error) {
	path := networkFilterPath(virtualMachineUuid, direction) + "/port"
	jsonBody := map[string]interface{}{
		"ports": ports,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

// DeleteNetworkFilterPorts removes ports from the packet filter of one direction of a virtual
// machine.
func DeleteNetworkFilterPorts(context AccessContext, virtualMachineUuid, direction string, ports []string) (map[string]interface{}, error) {
	path := networkFilterPath(virtualMachineUuid, direction) + "/port"
	jsonBody := map[string]interface{}{
		"ports": ports,
	}
	return SendDeleteWithBody(context, context.HanamiAddress, path, jsonBody)
}

func GetNetworkFilter(context AccessContext, virtualMachineUuid, direction string) (map[string]interface{}, error) {
	path := networkFilterPath(virtualMachineUuid, direction)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListNetworkFilter(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/network_filter"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

// DeleteNetworkFilter removes the whole packet filter of one direction of a virtual machine.
func DeleteNetworkFilter(context AccessContext, virtualMachineUuid, direction string) (map[string]interface{}, error) {
	path := networkFilterPath(virtualMachineUuid, direction)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
