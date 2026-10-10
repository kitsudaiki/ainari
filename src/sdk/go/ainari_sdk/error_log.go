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
	"net/url"
)

// LogListFilter contains the optional filters for listing the entries of the error- and
// audit-log of omikuji. Empty values and a Limit of 0 are not sent, so omikuji returns the
// newest 100 entries of all projects, users and hosts.
type LogListFilter struct {
	ProjectId string
	UserId    string
	HostName  string
	Limit     int
}

// vars converts the filter into the query-parameters of the request.
func (filter LogListFilter) vars() map[string]interface{} {
	vars := map[string]interface{}{}
	if filter.ProjectId != "" {
		vars["project_id"] = url.QueryEscape(filter.ProjectId)
	}
	if filter.UserId != "" {
		vars["user_id"] = url.QueryEscape(filter.UserId)
	}
	if filter.HostName != "" {
		vars["host_name"] = url.QueryEscape(filter.HostName)
	}
	if filter.Limit > 0 {
		vars["limit"] = filter.Limit
	}
	return vars
}

// GetErrorLog returns a single entry of the error-log of omikuji. Only admins are allowed to
// read it.
func GetErrorLog(context AccessContext, entryId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/error_log/%s/admin", entryId)
	vars := map[string]interface{}{}
	return SendGet(context, context.OmikujiAddress, path, vars)
}

// ListErrorLog returns the entries of the error-log of omikuji, sorted by their creation-time
// with the newest first. Only admins are allowed to read it.
func ListErrorLog(context AccessContext, filter LogListFilter) (map[string]interface{}, error) {
	path := "v1alpha/error_log/admin"
	return SendGet(context, context.OmikujiAddress, path, filter.vars())
}
