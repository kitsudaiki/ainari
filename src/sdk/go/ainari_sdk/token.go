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
	//b64 "encoding/base64"
)

func RequestToken(address, user, passphrase string, skipTlsVerification bool) string {
	path := "v1alpha/token"
	//b64.StdEncoding.EncodeToString([]byte(passphrase))
	var body = fmt.Sprintf("token_format=jwt&grant_type=client_credentials&client_id=%s&client_secret=%s", user, passphrase)

	content, err := sendAuthRequest(address, path, body, skipTlsVerification)
	if err != nil {
		return ""
	}

	return content["access_token"].(string)
}
