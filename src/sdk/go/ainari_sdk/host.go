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

func GetHost(context AccessContext, hostId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s/admin", hostId)
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func ListHost(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/host/admin"
	vars := map[string]interface{}{}
	return SendGet(context, context.HanamiAddress, path, vars)
}

func DeleteHost(context AccessContext, hostId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s/admin", hostId)
	vars := map[string]interface{}{}
	return SendDelete(context, context.HanamiAddress, path, vars)
}

// GetOnsenHost returns information of a specific onsen-host, which stores images and snapshots.
// SetHostIsolation sets, if a sakura-host is isolated, so it is only used by the virtual machines
// of a single project. This is only possible, while no virtual machine runs on the host.
func SetHostIsolation(context AccessContext, hostId string, isHostIsolated bool) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s/isolation/admin", hostId)
	jsonBody := map[string]interface{}{
		"is_host_isolated": isHostIsolated,
	}
	return SendPut(context, context.HanamiAddress, path, jsonBody)
}

func GetOnsenHost(context AccessContext, hostId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s/admin", hostId)
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

func ListOnsenHost(context AccessContext) (map[string]interface{}, error) {
	path := "v1alpha/host/admin"
	vars := map[string]interface{}{}
	return SendGet(context, context.RyokanAddress, path, vars)
}

// SetOnsenHostBackupTarget sets, if an onsen-host is a backup-target, which only stores the
// backups of isekai and never normal images or snapshots. A host can only be marked, while no
// images or snapshots are stored on it, and only be unmarked, while no backups are stored on it.
func SetOnsenHostBackupTarget(context AccessContext, hostId string, isBackupTarget bool) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s/backup_target/admin", hostId)
	jsonBody := map[string]interface{}{
		"is_backup_target": isBackupTarget,
	}
	return SendPut(context, context.RyokanAddress, path, jsonBody)
}

func DeleteOnsenHost(context AccessContext, hostId string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/host/%s/admin", hostId)
	vars := map[string]interface{}{}
	return SendDelete(context, context.RyokanAddress, path, vars)
}
