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
	"crypto/tls"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"strings"
	"bytes"
	"io"
	"os"
	"mime/multipart"
	"path/filepath"
)

const chunkSize = 1024 * 1024 // 1 MiB

type RequestError struct {
	StatusCode int
	Err        string
}

func (r *RequestError) Error() string {
	return fmt.Sprintf("status %d: err %v", r.StatusCode, r.Err)
}

func SendPost(context AccessContext, address, path string, jsonBody map[string]interface{}) (map[string]interface{}, error) {
	return sendGenericRequest(address, context.token, "POST", path, &jsonBody, context.skipTlsVerification)
}

func SendPut(context AccessContext, address, path string, jsonBody map[string]interface{}) (map[string]interface{}, error) {
	return sendGenericRequest(address, context.token, "PUT", path, &jsonBody, context.skipTlsVerification)
}

func SendGet(context AccessContext, address, path string, vars map[string]interface{}) (map[string]interface{}, error) {
	completePath := path + prepareVars(vars)
	return sendGenericRequest(address, context.token, "GET", completePath, nil, context.skipTlsVerification)
}

func SendDelete(context AccessContext, address, path string, vars map[string]interface{}) (map[string]interface{}, error) {
	completePath := path + prepareVars(vars)
	return sendGenericRequest(address, context.token, "DELETE", completePath, nil, context.skipTlsVerification)
}

func prepareVars(vars map[string]interface{}) string {
	if len(vars) > 0 {
		var pairs []string
		for key, value := range vars {
			if strVal, ok := value.(string); ok {
				pairs = append(pairs, fmt.Sprintf("%s=%s", key, strVal))
			} else {
				str := fmt.Sprintf("%v", value)
				pairs = append(pairs, fmt.Sprintf("%s=%s", key, str))
			}

		}
		return fmt.Sprintf("?%s", strings.Join(pairs, "&"))
	}

	return ""
}

func sendAuthRequest(address, path string, body string, skipTlsVerification bool) (map[string]interface{}, error) {
	outputMap := map[string]interface{}{}

	// check if https or not
	if strings.Contains(address, "https") {
		http.DefaultTransport.(*http.Transport).TLSClientConfig = &tls.Config{InsecureSkipVerify: skipTlsVerification}
	}

	// build uri
	var reqBody = strings.NewReader(body)
	completePath := fmt.Sprintf("%s/%s", address, path)
	// fmt.Println("completePath: " + completePath)
	// fmt.Println("request-body: " + jsonDataStr)
	req, err := http.NewRequest("POST", completePath, reqBody)
	if err != nil {
		return outputMap, err
	}

	// run request
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return outputMap, err
	}
	defer resp.Body.Close()

	// read data from response and convert into string
	bodyBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return outputMap, err
	}
	bodyString := string(bodyBytes)
	// fmt.Printf("bodyString: " + bodyString + "\n")

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return outputMap, &RequestError{
			StatusCode: resp.StatusCode,
			Err:        bodyString,
		}
	}
	
	// parse result
	err = json.Unmarshal([]byte(bodyString), &outputMap)
	if err != nil {
		return outputMap, err
	}

	return outputMap, nil
}

func sendGenericRequest(address, token, requestType, path string, jsonBody *map[string]interface{}, skipTlsVerification bool) (map[string]interface{}, error) {
	outputMap := map[string]interface{}{}
	jsonDataStr := ""
	if jsonBody != nil {
		jsonData, err := json.Marshal(jsonBody)
		if err != nil {
			return outputMap, err
		}
		jsonDataStr = string(jsonData)
	}
	// fmt.Println("request-body: "+ jsonDataStr + "\n")

	// check if https or not
	if strings.Contains(address, "https") {
		http.DefaultTransport.(*http.Transport).TLSClientConfig = &tls.Config{InsecureSkipVerify: skipTlsVerification}
	}

	// build uri
	var reqBody = strings.NewReader(jsonDataStr)
	completePath := fmt.Sprintf("%s/%s", address, path)
	// fmt.Println("completePath: " + completePath)
	// fmt.Println("request-body: " + jsonDataStr)
	req, err := http.NewRequest(requestType, completePath, reqBody)
	if err != nil {
		return outputMap, err
	}

	// add token to header
	var bearer_token = fmt.Sprintf("Bearer %s", token)
	req.Header.Set("Authorization",  bearer_token)
	req.Header.Set("Content-Type", "application/json")

	// run request
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return outputMap, err
	}
	defer resp.Body.Close()

	// read data from response and convert into string
	bodyBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return outputMap, err
	}
	bodyString := string(bodyBytes)

	// fmt.Printf("response-body: " + bodyString + "\n")
	// fmt.Printf("resp.StatusCode: %s", resp.StatusCode)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return outputMap, &RequestError{
			StatusCode: resp.StatusCode,
			Err:        bodyString,
		}
	}

	// parse result
	_ = json.Unmarshal([]byte(bodyString), &outputMap)
	return outputMap, nil
}

func UploadFiles(context AccessContext, path string, filePaths []string) (map[string]interface{}, error) {
	body := &bytes.Buffer{}
	writer := multipart.NewWriter(body)
	outputMap := map[string]interface{}{}

	// Open and stream each file into multipart writer
	for idx, path := range filePaths {
		file, err := os.Open(path)
		if err != nil {
			return outputMap, fmt.Errorf("failed to open file %s: %v", path, err)
		}
		defer file.Close()

		part, err := writer.CreateFormFile(fmt.Sprintf("file%d", idx), filepath.Base(path))
		if err != nil {
			return outputMap, fmt.Errorf("failed to create form part for %s: %v", path, err)
		}

		buf := make([]byte, chunkSize)
		for {
			n, err := file.Read(buf)
			if err != nil && err != io.EOF {
				return outputMap, fmt.Errorf("error reading file %s: %v", path, err)
			}
			if n == 0 {
				break
			}
			if _, err := part.Write(buf[:n]); err != nil {
				return outputMap, fmt.Errorf("error writing part for %s: %v", path, err)
			}
		}
	}

	// Close the writer to finish the form
	if err := writer.Close(); err != nil {
		return outputMap, fmt.Errorf("failed to close writer: %w", err)
	}

	// check if https or not
	if strings.Contains(context.RyokanAddress, "https") {
		http.DefaultTransport.(*http.Transport).TLSClientConfig = &tls.Config{InsecureSkipVerify: context.skipTlsVerification}
	}

	// Create the request
	completePath := fmt.Sprintf("%s/%s", context.RyokanAddress, path)
	// fmt.Println("completePath: " + completePath)
	// fmt.Println("request-body: " + jsonDataStr)
	req, err := http.NewRequest("POST", completePath, body)
	if err != nil {
		return outputMap, fmt.Errorf("failed to create request: %w", err)
	}

	var bearer_token = fmt.Sprintf("Bearer %s", context.token)
	req.Header.Set("Authorization",  bearer_token)
	req.Header.Set("Content-Type", writer.FormDataContentType())

	// Send the request
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return outputMap, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	// Read and print the response (optional!)
	bodyBytes, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return outputMap, err
	}
	bodyString := string(bodyBytes)

	// fmt.Printf("response-body: " + bodyString + "\n")
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return outputMap, &RequestError{
			StatusCode: resp.StatusCode,
			Err:        bodyString,
		}
	}

	// parse result
	_ = json.Unmarshal([]byte(bodyString), &outputMap)
	return outputMap, nil
}
