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

// const chunkSize = 128 * 1024 // 128 KiB

func CreateMnistDataset(context AccessContext, datasetName, imageFilePath, labelFilePath string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/dataset/mnist/%s", datasetName)
	files := []string{imageFilePath, labelFilePath}
	return UploadFiles(context, path, files)
}

func CreateCsvDataset(context AccessContext, datasetName, filePath string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/dataset/csv/%s", datasetName)
	files := []string{filePath}
	return UploadFiles(context, path, files)
}

func GetDataset(context AccessContext, datasetUuid string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/dataset/%s", datasetUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func ListDataset(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/dataset"
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func DeleteDataset(context AccessContext, datasetUuid string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/dataset/%s", datasetUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.RyokanAddress, path, vars)
}

func CheckDataset(context AccessContext, uuid, referenceDatasetUuid string,) (map[string]interface{}, error) {
	path := "v1alpha/dataset/check"
	vars := map[string]interface{}{
		"uuid":           uuid,
		"reference_uuid": referenceDatasetUuid,
	}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func DownloadDatasetContent(context AccessContext, datasetUuid, columnName string, numberOfRows, rowOffset int,) (map[string]interface{}, error) {
	path := "v1alpha/dataset/content"
	vars := map[string]interface{}{
		"uuid":           datasetUuid,
		"column_name":    columnName,
		"number_of_rows": numberOfRows,
		"row_offset":     rowOffset,
	}
	return SendGet(context, context.RyokanAddress, path, vars)
}
