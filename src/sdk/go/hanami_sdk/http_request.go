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
	"crypto/tls"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"strings"
)

type RequestError struct {
	StatusCode int
	Err        string
}

func (r *RequestError) Error() string {
	return fmt.Sprintf("status %d: err %v", r.StatusCode, r.Err)
}

func SendPost(address, token, path string, jsonBody map[string]interface{}, skipTlsVerification bool) (map[string]interface{}, error) {
	return sendGenericRequest(address, token, "POST", path, &jsonBody, skipTlsVerification)
}

func SendPut(address, token, path string, jsonBody map[string]interface{}, skipTlsVerification bool) (map[string]interface{}, error) {
	return sendGenericRequest(address, token, "PUT", path, &jsonBody, skipTlsVerification)
}

func SendGet(address, token, path string, vars map[string]interface{}, skipTlsVerification bool) (map[string]interface{}, error) {
	completePath := path + prepareVars(vars)
	return sendGenericRequest(address, token, "GET", completePath, nil, skipTlsVerification)
}

func SendDelete(address, token, path string, vars map[string]interface{}, skipTlsVerification bool) (map[string]interface{}, error) {
	completePath := path + prepareVars(vars)
	return sendGenericRequest(address, token, "DELETE", completePath, nil, skipTlsVerification)
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
	//fmt.Printf("bodyString: " + bodyString + "\n")

	if resp.StatusCode != http.StatusOK {
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

	fmt.Printf("bodyString: " + bodyString + "\n")
	if resp.StatusCode != http.StatusOK {
		return outputMap, &RequestError{
			StatusCode: resp.StatusCode,
			Err:        bodyString,
		}
	}

	// parse result
	err = json.Unmarshal([]byte(bodyString), &outputMap)
	if err != nil {
		return outputMap, nil
	}

	return outputMap, nil
}
