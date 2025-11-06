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

func CreateProject(context AccessContext, projectId, projectName string) (map[string]interface{}, error) {
	path := "v1alpha/project/admin"
	jsonBody := map[string]interface{}{
		"id":   projectId,
		"name": projectName,
	}
	return SendPost(context, context.MikoAddress, path, jsonBody)
}

func GetProject(context AccessContext, projectId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/project/%s/admin", projectId)
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

func ListProject(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/project/admin"
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

func DeleteProject(context AccessContext, projectId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/project/%s/admin", projectId)
	vars := map[string]interface{}{}
	return SendDelete(context, context.MikoAddress, path, vars)
}
