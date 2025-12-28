/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>
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

func CreateSecret(context AccessContext, secretName, secretPayload string) (map[string]interface{}, error) {
	path := "v1alpha/secret"
	jsonBody := map[string]interface{}{
		"name":           secretName,
		"secret_payload": secretPayload,
	}
	return SendPost(context, context.OmamoriAddress, path, jsonBody)
}

func GetSecret(context AccessContext, secretUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/secret/%s", secretUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.OmamoriAddress, path, vars)
}

func GetSecretWitchPayload(context AccessContext, secretUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/secret/%s/payload", secretUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.OmamoriAddress, path, vars)
}

func ListSecret(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/secret"
	vars := map[string]interface{}{}
	return SendGet(context, context.OmamoriAddress, path, vars)
}

func DeleteSecret(context AccessContext, secretUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/secret/%s", secretUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.OmamoriAddress, path, vars)
}
