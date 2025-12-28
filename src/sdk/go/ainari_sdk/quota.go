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
)

func GetQuota(context AccessContext, userId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/quota/%s/admin", userId)
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

func ListQuota(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/quota/admin"
	vars := map[string]interface{}{}
	return SendGet(context, context.MikoAddress, path, vars)
}

func SetQuota(context AccessContext, userId string, maxCluster, maxDataset, maxCheckpoint, maxSecret, maxTaskqueue int) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/quota/%s/admin", userId)
	jsonBody := map[string]interface{}{
		"max_cluster":    maxCluster,
		"max_dataset":    maxDataset,
		"max_checkpoint": maxCheckpoint,
		"max_secret":     maxSecret,
		"max_taskqueue":  maxTaskqueue,
	}
	return SendPut(context, context.MikoAddress, path, jsonBody)
}
