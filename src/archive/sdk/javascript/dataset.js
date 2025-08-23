// Apache License Version 2.0

// Copyright 2020 Tobias Anker

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

function sendFile(websocket, file, uuid, fileUuid)
{
    protobuf.load("/ainari_messages/protobuffers/ainari_messages.proto3", function(err, root) 
    {
        if(err) {
            throw err;
        }

        // Obtain a message type
        let FileUpload_Message = root.lookupType("FileUpload_Message");

        let segmentSize = 96 * 1024;
        let fileSize = file.size;

        for(let start = 0; start < fileSize; start += segmentSize) 
        {
            let reader = new FileReader();
            if(start + segmentSize > fileSize) {
                segmentSize = fileSize - start;
            }
            let blob = file.slice(start, segmentSize + start);
            let isLast = start + segmentSize >= fileSize;

            // read part of the file and send id as new message
            reader.onload = function(e) 
            {
                var payload = { 
                    datasetUuid: uuid, 
                    fileUuid: fileUuid, 
                    isLast: isLast,
                    type: 0,
                    position: start,
                    // get reeader-result and cut unnecessary  header of the resulting string
                    data: e.target.result.split(',')[1]
                };
                var errMsg = FileUpload_Message.verify(payload);
                if(errMsg) {
                    throw Error(errMsg);
                }

                // Create a new message and ncode a message to an Uint8Array (browser)
                let message = FileUpload_Message.create(payload);
                let buffer = FileUpload_Message.encode(message).finish();

                websocket.send(buffer);
            };

            // read as DataURL instead of array-buffer, 
            // because the javascript websocket seems to have problems with plain raw-data
            reader.readAsDataURL(blob);
        }
    });

    return true;
}

/**
 * wait for a specific amount of time
 *
 * @param {ms} number of milliseconds to sleep
 */
function sleep(ms) {
    var start = new Date().getTime(), expire = start + ms;
    while (new Date().getTime() < expire) { }
    return;
}

/**
 * wait until a data-set is fully uploaded
 *
 * @param {uuid} uuid of the dataset to check
 * @param {token} access jwt-token
 */
function waitUntilUploadComplete(uuid, token)
{
    // wait until upload completed
    var completeUploaded = false;
    while(completeUploaded == false)
    {
        // wait 500ms
        sleep(500);  
        var request = new XMLHttpRequest();
        // `false` makes the request synchronous
        request.open('GET', '/control/v1/dataset/progress?uuid=' + uuid, false); 
        request.setRequestHeader("X-Auth-Token", token);
        request.send(null);

        if(request.status !== 200) {
            return false;
        }

        const jsonContent = JSON.parse(request.responseText);
        completeUploaded = jsonContent.complete;
    }

    return true;
}

/**
 * wait until a data-set is fully uploaded
 *
 * @param {websocket} uuid of the dataset to check
 * @param {uuid} access jwt-token
 * @param {file} uuid of the dataset to check
 * @param {fileUuid} access jwt-token
 */
function sendCsvFiles(websocket, uuid, file, fileUuid)
{
    const token = getAndCheckToken();
    if(token == "") {
        return false;
    }
    
    websocket.onopen = function () {
        console.log("WebSocket open")
        const initialMsg = "{\"token\":\"" + token + "\",\"target\":\"shiori\"}";
        websocket.send(initialMsg);
    };
    
    websocket.onerror = function () {
        console.log("WebSocket failed!");
    };
    
    websocket.onmessage = function(event) {
        var reader = new FileReader();
        reader.onload = function() {
            console.log("Data received from server: " + reader.result);
        }
        reader.readAsText(event.data);
         
        if(sendFile(websocket, file, uuid, fileUuid) == false) {
            return false;
        }
    };

    if(waitUntilUploadComplete(uuid, token) == false) {
        return false;
    }

    return true;
}

/**
 * upload finalize the upload of the csv-dataset
 *
 * @param {uuid} uuid of the data-set
 * @param {inputUuid} uuid of the temporary file for the input-data
 * @param {token} access jwt-token
 */
function finishCsvUpload(uuid, inputUuid, token)
{
    // finish upload
    var request = new XMLHttpRequest();
    // `false` makes the request synchronous
    request.open('PUT', '/control/v1/csv/dataset', false);  
    request.setRequestHeader("X-Auth-Token", token);
    var jsonBody = "{\"uuid\":\"" + uuid 
                   + "\",\"uuid_input_file\":\"" + inputUuid + "\"}";
    request.send(jsonBody);

    if(request.status !== 200) {
        return false;
    }

    return true;
}

/**
 * send mnist-files over a websocket
 *
 * @param {websocket} websocket where the data should be send
 * @param {uuid} uuid of the data-set
 * @param {inputFile} file with input-data
 * @param {labelFile} file with label-data
 * @param {inputFileUuid} uuid of the temporary file for the input-data
 * @param {labelFileUuid} uuid of the temporary file for the label-data
 */
function sendMnistFiles(websocket, uuid, inputFile, labelFile, inputFileUuid, labelFileUuid)
{
    const token = getAndCheckToken();
    if(token == "") {
        return false;
    }
    
    websocket.onopen = function () {
        console.log("WebSocket open")
        const initialMsg = "{\"token\":\"" + token + "\",\"target\":\"shiori\"}";
        websocket.send(initialMsg);
    };
    
    websocket.onerror = function () {
        console.log("WebSocket failed!");
    };
    
    websocket.onmessage = function(event) {
        var reader = new FileReader();
        reader.onload = function() {
            console.log("Data received from server: " + reader.result);
        }
        reader.readAsText(event.data);
         
        if(sendFile(websocket, inputFile, uuid, inputFileUuid) == false) {
            return false;
        }
        if(sendFile(websocket, labelFile, uuid, labelFileUuid) == false) {
            return false;
        }
    };

    if(waitUntilUploadComplete(uuid, token) == false) {
        return false;
    }

    return true;
}

/**
 * upload finalize the upload of the mnist-dataset
 *
 * @param {uuid} uuid of the data-set
 * @param {inputUuid} uuid of the temporary file for the input-data
 * @param {labelUuid} uuid of the temporary file for the label-data
 * @param {token} access jwt-token
 */
function finishMnistUpload(uuid, inputUuid, labelUuid, token)
{
    // finish upload
    var request = new XMLHttpRequest();
    // `false` makes the request synchronous
    request.open('PUT', '/control/v1/mnist/dataset', false);  
    request.setRequestHeader("X-Auth-Token", token);
    var jsonBody = "{\"uuid\":\"" + uuid 
                   + "\",\"uuid_input_file\":\"" + inputUuid 
                   + "\",\"uuid_label_file\":\"" + labelUuid + "\"}";
    request.send(jsonBody);

    if(request.status !== 200) {
        return false;
    }

    return true;
}

function uploadMnistFiles(jsonContent, inputFile, labelFile, token)
{
    const uuid = jsonContent.uuid;
    const inputFileUuid = jsonContent.uuid_input_file;
    const labelFileUuid = jsonContent.uuid_label_file;

    var websocket = new WebSocket('wss://' + location.host);
    if(sendMnistFiles(websocket, uuid, inputFile, labelFile, inputFileUuid, labelFileUuid) == false) {
        return;
    }

    if(finishMnistUpload(uuid, inputFileUuid, labelFileUuid, token) == false) {
        return;
    }
}

function uploadCsvFile(jsonContent, inputFile, token)
{
    const uuid = jsonContent.uuid;
    const inputFileUuid = jsonContent.uuid_input_file;

    var websocket = new WebSocket('wss://' + location.host);
    if(sendCsvFiles(websocket, uuid, inputFile, inputFileUuid) == false) {
        return;
    }

    if(finishCsvUpload(uuid, inputFileUuid, token) == false) {
        return;
    }
}


function createMnistDataSet_request(outputFunc, name, inputFile, labelFile, token)
{
    const path = "/control/v1/mnist/dataset";
    let  reqContent = "{\"name\":\"" + name;
    reqContent += "\",\"input_data_size\":" + inputFile.size 
    reqContent += ",\"label_data_size\":" + labelFile.size + "}";
    createObject_request(outputFunc, path, reqContent, token);
}

function createCsvDataSet_request(outputFunc, name, inputFile, token)
{
    const path = "/control/v1/csv/dataset";
    let reqContent = "{\"name\":\"" + name;
    reqContent += "\",\"input_data_size\":" + inputFile.size + "}";
    createObject_request(outputFunc, path, reqContent, token);
}

function listDataSet_request(outputFunc, token)
{
    listObjects_request(outputFunc, "/control/v1/dataset/all", token);
}

function deleteDataSet_request(postProcessFunc, dataSetUuid, token)
{
    const request = "/control/v1/dataset?uuid=" + dataSetUuid;
    deleteObject_request(postProcessFunc, request, token);
}

