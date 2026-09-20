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

// UploadPublicKey uploads a new ssh-public-key to omamori. The fingerprint of the key is
// calculated by the backend and doesn't have to be provided here.
func UploadPublicKey(context AccessContext, publicKeyName, publicKey string) (map[string]interface{}, error) {
	path := "v1alpha/public_key"
	jsonBody := map[string]interface{}{
		"name":       publicKeyName,
		"public_key": publicKey,
	}
	return SendPost(context, context.OmamoriAddress, path, jsonBody)
}

func GetPublicKey(context AccessContext, publicKeyUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/public_key/%s", publicKeyUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.OmamoriAddress, path, vars)
}

func ListPublicKey(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/public_key"
	vars := map[string]interface{}{}
	return SendGet(context, context.OmamoriAddress, path, vars)
}

func DeletePublicKey(context AccessContext, publicKeyUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/public_key/%s", publicKeyUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.OmamoriAddress, path, vars)
}
