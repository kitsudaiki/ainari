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

// Package ainari_sdk provides utilities for interacting with the Ainari API.
// It handles authentication, request sending, and file uploads to the Ainari service.
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
	"sort"
)

// secureClient and insecureClient have their own transports, so the TLS-settings of the SDK
// never change http.DefaultTransport, which is shared with the rest of the process, which embeds
// the SDK.
var (
	secureClient   = newHTTPClient(false)
	insecureClient = newHTTPClient(true)
)

// newHTTPClient creates a client with a copy of the default transport, which only differs in the
// verification of the TLS-certificates.
func newHTTPClient(skipTlsVerification bool) *http.Client {
	transport := http.DefaultTransport.(*http.Transport).Clone()
	transport.TLSClientConfig = &tls.Config{InsecureSkipVerify: skipTlsVerification}
	return &http.Client{Transport: transport}
}

// httpClient returns the client for the requested verification of the TLS-certificates.
func httpClient(skipTlsVerification bool) *http.Client {
	if skipTlsVerification {
		return insecureClient
	}
	return secureClient
}

// RequestError represents an error that occurs during an HTTP request.
// It contains the HTTP status code and the error message from the response.
type RequestError struct {
	StatusCode int
	Err        string
}

// Error implements the error interface for RequestError.
func (r *RequestError) Error() string {
	return fmt.Sprintf("status %d: err %v", r.StatusCode, r.Err)
}

// SendPost sends a POST request to the specified path with the given JSON body.
// It uses the provided AccessContext for authentication and TLS configuration.
func SendPost(context AccessContext, address, path string, jsonBody map[string]interface{}) (map[string]interface{}, error) {
	return sendGenericRequest(address, context.token, "POST", path, &jsonBody, context.skipTlsVerification)
}

// SendPut sends a PUT request to the specified path with the given JSON body.
// It uses the provided AccessContext for authentication and TLS configuration.
func SendPut(context AccessContext, address, path string, jsonBody map[string]interface{}) (map[string]interface{}, error) {
	return sendGenericRequest(address, context.token, "PUT", path, &jsonBody, context.skipTlsVerification)
}

// SendGet sends a GET request to the specified path with the given variables.
// It uses the provided AccessContext for authentication and TLS configuration.
func SendGet(context AccessContext, address, path string, vars map[string]interface{}) (map[string]interface{}, error) {
	completePath := path + prepareVars(vars)
	return sendGenericRequest(address, context.token, "GET", completePath, nil, context.skipTlsVerification)
}

// SendDelete sends a DELETE request to the specified path with the given variables.
// It uses the provided AccessContext for authentication and TLS configuration.
func SendDelete(context AccessContext, address, path string, vars map[string]interface{}) (map[string]interface{}, error) {
	completePath := path + prepareVars(vars)
	return sendGenericRequest(address, context.token, "DELETE", completePath, nil, context.skipTlsVerification)
}

// SendDeleteWithBody sends a DELETE request to the specified path with the given JSON body.
// It is used by the endpoints which describe what has to be removed in the body instead of
// addressing it completely by the path.
// It uses the provided AccessContext for authentication and TLS configuration.
func SendDeleteWithBody(context AccessContext, address, path string, jsonBody map[string]interface{}) (map[string]interface{}, error) {
	return sendGenericRequest(address, context.token, "DELETE", path, &jsonBody, context.skipTlsVerification)
}

// prepareVars constructs a query string from the given map of variables.
// It converts each value to a string and joins them with '&' characters.
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

// sendAuthRequest sends an authentication request to the specified path with the given body.
// It handles TLS configuration and returns the response as a map.
func sendAuthRequest(address, path string, body string, skipTlsVerification bool) (map[string]interface{}, error) {
	outputMap := map[string]interface{}{}

	// build uri
	var reqBody = strings.NewReader(body)
	completePath := fmt.Sprintf("%s/%s", address, path)
	req, err := http.NewRequest("POST", completePath, reqBody)
	if err != nil {
		return outputMap, err
	}

	// run request
	resp, err := httpClient(skipTlsVerification).Do(req)
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

// sendGenericRequest sends a generic HTTP request to the specified path with the given body.
// It handles authentication, TLS configuration, and returns the response as a map.
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

	// build uri
	var reqBody = strings.NewReader(jsonDataStr)
	completePath := fmt.Sprintf("%s/%s", address, path)
	req, err := http.NewRequest(requestType, completePath, reqBody)
	if err != nil {
		return outputMap, err
	}

	// add token to header
	var bearer_token = fmt.Sprintf("Bearer %s", token)
	req.Header.Set("Authorization",  bearer_token)
	req.Header.Set("Content-Type", "application/json")

	// run request
	resp, err := httpClient(skipTlsVerification).Do(req)
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

	// handle error-response
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

// UploadProgressFunc is called during an upload with the number of bytes, which were already
// sent, and the total size of the request-body in bytes.
type UploadProgressFunc func(sent, total int64)

// progressReader wraps the reader of a request-body and reports, how much of it was already read
// by the http-client.
type progressReader struct {
	reader   io.Reader
	sent     int64
	total    int64
	progress UploadProgressFunc
}

func (r *progressReader) Read(p []byte) (int, error) {
	n, err := r.reader.Read(p)
	if n > 0 {
		r.sent += int64(n)
		r.progress(r.sent, r.total)
	}
	return n, err
}

// UploadFile uploads a file together with additional text-fields as multipart-form to the
// specified path. Fields with an empty value are not sent. The file is sent as field "file".
// It uses the provided AccessContext for authentication and TLS configuration.
// The file is streamed from disk, instead of loading it into memory. If progress is not nil, it
// is called while the request-body is sent.
func UploadFile(context AccessContext, path string, fields map[string]string, filePath string, progress UploadProgressFunc) (map[string]interface{}, error) {
	outputMap := map[string]interface{}{}

	file, err := os.Open(filePath)
	if err != nil {
		return outputMap, fmt.Errorf("failed to open file %s: %v", filePath, err)
	}
	defer file.Close()
	fileInfo, err := file.Stat()
	if err != nil {
		return outputMap, fmt.Errorf("failed to get size of file %s: %v", filePath, err)
	}

	// The text-fields and the header of the file-part are written in front of the file and the
	// closing boundary behind it. So the file can be streamed between them and the size of the
	// whole body is known in advance, which is required for the progress.
	form := &bytes.Buffer{}
	writer := multipart.NewWriter(form)

	// write the text-fields in a fixed order before the file
	keys := make([]string, 0, len(fields))
	for key := range fields {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	for _, key := range keys {
		if fields[key] == "" {
			continue
		}
		if err := writer.WriteField(key, fields[key]); err != nil {
			return outputMap, fmt.Errorf("failed to write form field %s: %v", key, err)
		}
	}
	if _, err := writer.CreateFormFile("file", filepath.Base(filePath)); err != nil {
		return outputMap, fmt.Errorf("failed to create form part for %s: %v", filePath, err)
	}
	head := bytes.Clone(form.Bytes())

	// Close the writer to get the closing boundary of the form
	form.Reset()
	if err := writer.Close(); err != nil {
		return outputMap, fmt.Errorf("failed to close writer: %w", err)
	}
	tail := bytes.Clone(form.Bytes())

	totalSize := int64(len(head)) + fileInfo.Size() + int64(len(tail))
	var body io.Reader = io.MultiReader(bytes.NewReader(head), file, bytes.NewReader(tail))
	if progress != nil {
		body = &progressReader{reader: body, total: totalSize, progress: progress}
	}

	// Create the request
	completePath := fmt.Sprintf("%s/%s", context.RyokanAddress, path)
	req, err := http.NewRequest("POST", completePath, body)
	if err != nil {
		return outputMap, fmt.Errorf("failed to create request: %w", err)
	}
	req.ContentLength = totalSize

	var bearer_token = fmt.Sprintf("Bearer %s", context.token)
	req.Header.Set("Authorization",  bearer_token)
	req.Header.Set("Content-Type", writer.FormDataContentType())

	// Send the request
	resp, err := httpClient(context.skipTlsVerification).Do(req)
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
