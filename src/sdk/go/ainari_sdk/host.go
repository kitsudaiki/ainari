/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
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

func GetHost(context AccessContext, hostId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s", hostId)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListHost(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/host"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func DeleteHost(context AccessContext, hostId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s", hostId)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
