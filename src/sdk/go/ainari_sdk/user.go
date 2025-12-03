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
