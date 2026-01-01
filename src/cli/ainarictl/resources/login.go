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

package ainari_resources

import (
	ainarictl_common "ainarictl/common"
	"os"

	ainari_sdk "github.com/kitsudaiki/ainari"
)

func Login() (ainari_sdk.AccessContext, error) {

	user := os.Getenv("AINARI_USER")
	passphrase := os.Getenv("AINARI_PASSPHRASE")
	address := os.Getenv("AINARI_ADDRESS")

	if user == "" {
		panic("AINARI_USER is not set")
	}
	if passphrase == "" {
		panic("AINARI_PASSPHRASE is not set")
	}
	if address == "" {
		panic("AINARI_ADDRESS is not set")
	}

	return ainari_sdk.RequestContext(address, user, passphrase, ainarictl_common.DisableTlsVerification)
}
