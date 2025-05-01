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

package hanami_sdk

import (
	"fmt"
	b64 "encoding/base64"
)

func CreateUser(address, token, userId, userName, passphrase string, is_admin, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/user"
	jsonBody := map[string]interface{}{
		"id":         userId,
		"name":       userName,
		"passphrase": b64.StdEncoding.EncodeToString([]byte(passphrase)),
		"is_admin":   is_admin,
	}
	return SendPost(address, token, path, jsonBody, skipTlsVerification)
}

func GetUser(address, token, userId string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/user/%s", userId)
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func ListUser(address, token string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/user"
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func DeleteUser(address, token, userId string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/user"
	vars := map[string]interface{}{"id": userId}
	return SendDelete(address, token, path, vars, skipTlsVerification)
}
