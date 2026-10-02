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
	// b64 "encoding/base64"
)

func CreateUser(context AccessContext, userId, userName, passphrase string, is_admin bool) (map[string]interface{}, error) {
	// "passphrase": b64.StdEncoding.EncodeToString([]byte(passphrase)),
	path := "v1alpha/user/admin"
	is_admin_str := "false"
	if is_admin {
		is_admin_str = "true"
	}
	jsonBody := map[string]interface{}{
		"id":         userId,
		"name":       userName,
		"passphrase": passphrase,
		"is_admin":   is_admin_str,
	}
	return SendPost(context, context.MikoAddress, path, jsonBody)
}

func GetUser(context AccessContext, userId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/user/%s/admin", userId)
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

func ListUser(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/user/admin"
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

func DeleteUser(context AccessContext, userId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/user/%s/admin", userId)
	vars := map[string]interface{}{}
	return SendDelete(context, context.MikoAddress, path, vars)
}

// AssignProject assigns a project with the given role to a user.
// Valid roles are "admin", "member" and "observer".
func AssignProject(context AccessContext, userId, projectId, projectRole string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/user/%s/assign_project/admin", userId)
	jsonBody := map[string]interface{}{
		"project_id":   projectId,
		"project_role": projectRole,
	}
	return SendPost(context, context.MikoAddress, path, jsonBody)
}

// UnassignProject removes the assignment of a project from a user.
func UnassignProject(context AccessContext, userId, projectId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/user/%s/unassign_project/admin", userId)
	jsonBody := map[string]interface{}{
		"project_id": projectId,
	}
	return SendPost(context, context.MikoAddress, path, jsonBody)
}

// SetProjectRole changes the role of a user within a project, to which the user is already assigned.
// Valid roles are "admin", "member" and "observer".
func SetProjectRole(context AccessContext, userId, projectId, projectRole string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/user/%s/set_project_role/admin", userId)
	jsonBody := map[string]interface{}{
		"project_id":   projectId,
		"project_role": projectRole,
	}
	return SendPut(context, context.MikoAddress, path, jsonBody)
}

// ListInvitedProjects returns all projects, to which the user of the current access-context is
// assigned, together with the role of the user within each of these projects.
func ListInvitedProjects(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/user/invited_projects"
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}
