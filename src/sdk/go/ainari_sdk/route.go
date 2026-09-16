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

func AddRoute(context AccessContext, destIp, targetIface, gatewayIp, nextHopIp, nextHopMac string, encrypted bool) (map[string]interface{}, error) {
	path := "v1alpha/route/internal"
	jsonBody := map[string]interface{}{
		"dest_ip":      destIp,
		"target_iface": targetIface,
		"encrypted":    encrypted,
	}
	// a route without a remote gateway is delivered locally
	if gatewayIp != "" {
		jsonBody["gateway_ip"] = gatewayIp
	}
	// the next-hop is resolved by the gateway itself, if it is not given explicitly
	if nextHopIp != "" {
		jsonBody["next_hop_ip"] = nextHopIp
	}
	if nextHopMac != "" {
		jsonBody["next_hop_mac"] = nextHopMac
	}
	return SendPost(context, context.ToriiAddress, path, jsonBody)
}

func ListRoute(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/route"
	vars := map[string]interface{}{}
	return SendGet(context, context.ToriiAddress, path, vars)
}

func UpdateRoute(context AccessContext, routeUuid, destIp, targetIface, gatewayIp, nextHopIp, nextHopMac string, encrypted bool) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/route/%s/internal", routeUuid)
	jsonBody := map[string]interface{}{
		"dest_ip":      destIp,
		"target_iface": targetIface,
		"encrypted":    encrypted,
	}
	// a route without a remote gateway is delivered locally
	if gatewayIp != "" {
		jsonBody["gateway_ip"] = gatewayIp
	}
	// the next-hop is resolved by the gateway itself, if it is not given explicitly
	if nextHopIp != "" {
		jsonBody["next_hop_ip"] = nextHopIp
	}
	if nextHopMac != "" {
		jsonBody["next_hop_mac"] = nextHopMac
	}
	return SendPut(context, context.ToriiAddress, path, jsonBody)
}

func DeleteRoute(context AccessContext, routeUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/route/%s/internal", routeUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.ToriiAddress, path, vars)
}
