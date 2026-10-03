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

// ListUsersInProject returns all users of the project of the current access-context, together
// with the role of each user within this project.
func ListUsersInProject(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/project/users"
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

// ListUsersInProjectAdmin returns all users of the given project, together with the role of
// each user within this project.
func ListUsersInProjectAdmin(context AccessContext, projectId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/project/%s/users/admin", projectId)
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

// AddUserToProject adds a user with the given role to a project.
// Valid roles are "admin", "member" and "observer".
func AddUserToProject(context AccessContext, projectId, userId, projectRole string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/project/%s/add_user/admin", projectId)
	jsonBody := map[string]interface{}{
		"user_id":      userId,
		"project_role": projectRole,
	}
	return SendPost(context, context.MikoAddress, path, jsonBody)
}

// RemoveUserFromProject removes a user from a project.
func RemoveUserFromProject(context AccessContext, projectId, userId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/project/%s/remove_user/admin", projectId)
	jsonBody := map[string]interface{}{
		"user_id": userId,
	}
	return SendPost(context, context.MikoAddress, path, jsonBody)
}
