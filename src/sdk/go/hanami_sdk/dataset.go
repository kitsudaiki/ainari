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


const chunkSize = 128 * 1024 // 128 KiB

func GetDataset(address, token, datasetUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/dataset"
	vars := map[string]interface{}{"uuid": datasetUuid}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func ListDataset(address, token string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/dataset/all"
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func DeleteDataset(address, token, datasetUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/dataset"
	vars := map[string]interface{}{"uuid": datasetUuid}
	return SendDelete(address, token, path, vars, skipTlsVerification)
}

func CheckDataset(address, token, uuid, referenceDatasetUuid string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/dataset/check"
	vars := map[string]interface{}{
		"uuid":           uuid,
		"reference_uuid": referenceDatasetUuid,
	}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func DownloadDatasetContent(address, token, datasetUuid, columnName string, numberOfRows, rowOffset int, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "v1alpha/dataset/content"
	vars := map[string]interface{}{
		"uuid":           datasetUuid,
		"column_name":    columnName,
		"number_of_rows": numberOfRows,
		"row_offset":     rowOffset,
	}
	return SendGet(address, token, path, vars, skipTlsVerification)
}
