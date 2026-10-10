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

// GetAuditLog returns a single entry of the audit-log of omikuji. Only admins are allowed to
// read it.
func GetAuditLog(context AccessContext, entryId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/audit_log/%s/admin", entryId)
	vars := map[string]interface{}{}
	return SendGet(context, context.OmikujiAddress, path, vars)
}

// ListAuditLog returns the entries of the audit-log of omikuji, sorted by their creation-time
// with the newest first. Only admins are allowed to read it.
func ListAuditLog(context AccessContext, filter LogListFilter) (map[string]interface{}, error) {
	path := "v1alpha/audit_log/admin"
	return SendGet(context, context.OmikujiAddress, path, filter.vars())
}
