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

// CreateDiskImage uploads a disk-image, which is used as boot-disk of a virtual machine. The file
// is stored as it is, so it has to be an image, which cloud-hypervisor can boot, like a
// qcow2-cloud-image.
func CreateDiskImage(context AccessContext, imageName, filePath string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/image/disk/%s", imageName)
	files := []string{filePath}
	return UploadFiles(context, path, files)
}

func CreateMnistImage(context AccessContext, imageName, imageFilePath, labelFilePath string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/image/mnist/%s", imageName)
	files := []string{imageFilePath, labelFilePath}
	return UploadFiles(context, path, files)
}

func CreateCsvImage(context AccessContext, imageName, filePath string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/image/csv/%s", imageName)
	files := []string{filePath}
	return UploadFiles(context, path, files)
}

func GetImage(context AccessContext, imageUuid string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/image/%s", imageUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func ListImage(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/image"
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func DeleteImage(context AccessContext, imageUuid string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/image/%s", imageUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.RyokanAddress, path, vars)
}

// CheckImage compares a column of an image with a column of a reference-image and returns the
// accuracy of the comparison.
func CheckImage(context AccessContext, imageUuid, imageColumn, referenceImageUuid, referenceColumn string,) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/image/%s/check", imageUuid)
	jsonBody := map[string]interface{}{
		"image_column":     imageColumn,
		"reference_uuid":   referenceImageUuid,
		"reference_column": referenceColumn,
	}
	return SendPut(context, context.RyokanAddress, path, jsonBody)
}

// GetImageCount returns the number of images of the project.
func GetImageCount(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/image/count"
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func DownloadImageContent(context AccessContext, imageUuid, columnName string, numberOfRows, rowOffset int,) (map[string]interface{}, error) {
	path := "v1alpha/image/content"
	vars := map[string]interface{}{
		"uuid":           imageUuid,
		"column_name":    columnName,
		"number_of_rows": numberOfRows,
		"row_offset":     rowOffset,
	}
	return SendGet(context, context.RyokanAddress, path, vars)
}
