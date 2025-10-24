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

func RequestContext(address, user, passphrase string, skipTlsVerification bool) (AccessContext, error){
	var context AccessContext

	path := "v1alpha/token"
	//b64.StdEncoding.EncodeToString([]byte(passphrase))
	var body = fmt.Sprintf("token_format=jwt&grant_type=client_credentials&client_id=%s&client_secret=%s", user, passphrase)

	content, err := sendAuthRequest(address, path, body, skipTlsVerification)
	if err != nil {
		return context, err
	}

	token := content["access_token"].(string)

	path = "v1alpha/endpoints"
	resp, err := sendGenericRequest(address, token, "GET", path, nil, skipTlsVerification)
	if err != nil {
		return context, err
	}

	sakuraAddr := resp["sakura"].(map[string]interface{})
	bentoAddr := resp["bento"].(map[string]interface{})

	context.token = token
	context.MikoAddress = address
	context.SakuraAddress = fmt.Sprintf("%s:%d", sakuraAddr["public_address"].(string), int(sakuraAddr["public_port"].(float64)))
	context.BentoAddress = fmt.Sprintf("%s:%d", bentoAddr["public_address"].(string), int(bentoAddr["public_port"].(float64)))
	context.skipTlsVerification = skipTlsVerification

	return context, nil
}
