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
	b64 "encoding/base64"
)

func CreateCluster(address, token, name, template string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/cluster"
	jsonBody := map[string]interface{}{
		"name":     name,
		"template": b64.StdEncoding.EncodeToString([]byte(template)),
	}
	return SendPost(address, token, path, jsonBody, skipTlsVerification)
}

func GetCluster(address, token, clusterUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/cluster"
	vars := map[string]interface{}{"uuid": clusterUuid}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func ListCluster(address, token string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/cluster/all"
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func DeleteCluster(address, token, clusterUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/cluster"
	vars := map[string]interface{}{"uuid": clusterUuid}
	return SendDelete(address, token, path, vars, skipTlsVerification)
}

func SaveCluster(address, token, clusterUuid, checkpointName string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/cluster/save"
	jsonBody := map[string]interface{}{
		"name":         checkpointName,
		"cluster_uuid": clusterUuid,
	}
	return SendPost(address, token, path, jsonBody, skipTlsVerification)
}

func RestoreCluster(address, token, clusterUuid, checkpointUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/cluster/load"
	jsonBody := map[string]interface{}{
		"checkpoint_uuid": checkpointUuid,
		"cluster_uuid":    clusterUuid,
	}
	return SendPost(address, token, path, jsonBody, skipTlsVerification)
}
