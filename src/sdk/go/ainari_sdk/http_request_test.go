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
	"bytes"
	"crypto/rand"
	"io"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

// The multipart-body of the upload is assembled by hand around the streamed file, so the server
// has to be able to parse it and the content-length has to match the sent body.
func TestUploadFile(t *testing.T) {
	content := make([]byte, 3*1024*1024+17)
	if _, err := rand.Read(content); err != nil {
		t.Fatal(err)
	}
	filePath := filepath.Join(t.TempDir(), "disk.img")
	if err := os.WriteFile(filePath, content, 0o600); err != nil {
		t.Fatal(err)
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v1alpha/image" {
			t.Errorf("unexpected path %s", r.URL.Path)
		}
		body, err := io.ReadAll(r.Body)
		if err != nil {
			t.Fatal(err)
		}
		if int64(len(body)) != r.ContentLength {
			t.Errorf("content-length %d doesn't match body of %d bytes", r.ContentLength, len(body))
		}
		r.Body = io.NopCloser(bytes.NewReader(body))
		if err := r.ParseMultipartForm(1 << 20); err != nil {
			t.Fatal(err)
		}
		if got := r.FormValue("name"); got != "my-image" {
			t.Errorf("name: got %q", got)
		}
		if got := r.FormValue("image_type"); got != "disk" {
			t.Errorf("image_type: got %q", got)
		}
		if _, ok := r.MultipartForm.Value["secret_uuid"]; ok {
			t.Errorf("empty secret_uuid was sent")
		}
		file, _, err := r.FormFile("file")
		if err != nil {
			t.Fatal(err)
		}
		uploaded, _ := io.ReadAll(file)
		if !bytes.Equal(uploaded, content) {
			t.Errorf("uploaded file differs from the original")
		}
		w.Write([]byte(`{"uuid": "abc"}`))
	}))
	defer server.Close()

	var lastSent, lastTotal int64
	calls := 0
	progress := func(sent, total int64) {
		if sent < lastSent {
			t.Errorf("progress went backwards from %d to %d", lastSent, sent)
		}
		lastSent, lastTotal = sent, total
		calls++
	}

	context := AccessContext{RyokanAddress: server.URL}
	resp, err := CreateDiskImage(context, "my-image", filePath, "", progress)
	if err != nil {
		t.Fatal(err)
	}
	if resp["uuid"] != "abc" {
		t.Errorf("unexpected response %v", resp)
	}
	if calls < 2 || lastSent != lastTotal || lastTotal <= int64(len(content)) {
		t.Errorf("progress incomplete: %d calls, %d of %d bytes", calls, lastSent, lastTotal)
	}
}
