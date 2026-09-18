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
func AddFloatingIp(context AccessContext, name, networkUuid, floatingIp, internalIp string) (map[string]interface{}, error) {
	path := "v1alpha/floating_ip"
	jsonBody := map[string]interface{}{
		"name":         name,
		"network_uuid": networkUuid,
		"internal_ip":  internalIp,
	}
	if floatingIp != "" {
		jsonBody["floating_ip"] = floatingIp
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

func DeleteFloatingIp(context AccessContext, floatingIp string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/floating_ip/%s", floatingIp)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
