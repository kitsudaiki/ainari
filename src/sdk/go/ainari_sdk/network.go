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

// CreateNetwork creates a new network with the given name for the given subnet in CIDR-notation.
func CreateNetwork(context AccessContext, name, subnet string) (map[string]interface{}, error) {
	path := "v1alpha/network"
	jsonBody := map[string]interface{}{
		"name":   name,
		"subnet": subnet,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

func GetNetwork(context AccessContext, networkUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/network/%s", networkUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListNetwork(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/network"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func DeleteNetwork(context AccessContext, networkUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/network/%s", networkUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
