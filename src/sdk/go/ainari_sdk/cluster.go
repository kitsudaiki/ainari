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

func CreateCluster(context AccessContext, name, template string) (map[string]interface{}, error) {
	path := "v1alpha/cluster"
	jsonBody := map[string]interface{}{
		"name":     name,
		"template": template,
	}
	return SendPost(context, context.HanamiAddress, path, jsonBody)
}

func GetCluster(context AccessContext, clusterUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/cluster/%s", clusterUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListCluster(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/cluster"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func DeleteCluster(context AccessContext, clusterUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/cluster/%s", clusterUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}
