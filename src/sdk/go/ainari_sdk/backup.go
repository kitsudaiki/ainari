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

// RestoreOptions are the flags of the restore of a backup. Without `Force`, resources, whose UUID
// is already used, get a new UUID. With `Force`, existing resources are overwritten. `Prune`
// deletes all resources of the project, which are not part of the backup, and is only allowed
// together with `Force`.
type RestoreOptions struct {
	Force               bool
	Prune               bool
	TargetProjectId     string
	StartVms            bool
	IncludeDependencies bool
	DryRun              bool
}

// CreateProjectBackup creates a backup of the whole project of the token. The backup is created by
// a job in the background, so the response contains the backup and the job, which can be polled.
// The project is locked, until the job is done. The passphrase is not stored, so the backup can't
// be restored without it.
func CreateProjectBackup(context AccessContext, name, passphrase string, stopVms, includeImages bool) (map[string]interface{}, error) {
	path := "v1alpha/backup"
	jsonBody := map[string]interface{}{
		"name":           name,
		"passphrase":     passphrase,
		"stop_vms":       stopVms,
		"include_images": includeImages,
	}
	return SendPost(context, context.IsekaiAddress, path, jsonBody)
}

// CreateVirtualMachineBackup creates a backup of a single virtual machine with its root-disk,
// public-key, network and packet-filters. Like the backup of a project, it is created by a job in
// the background.
func CreateVirtualMachineBackup(context AccessContext, virtualMachineUuid, name, passphrase string, stopVm bool) (map[string]interface{}, error) {
	path := "v1alpha/backup/virtual_machine"
	jsonBody := map[string]interface{}{
		"virtual_machine_uuid": virtualMachineUuid,
		"name":                 name,
		"passphrase":           passphrase,
		"stop_vm":              stopVm,
	}
	return SendPost(context, context.IsekaiAddress, path, jsonBody)
}

func GetBackup(context AccessContext, backupUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/backup/%s", backupUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.IsekaiAddress, path, vars)
}

// ListBackup lists the backups of the project of the token. Admins see the backups of all
// projects, which can be restricted to one project with projectId. An empty projectId lists all.
func ListBackup(context AccessContext, projectId string) (map[string]interface{}, error) {
	path := "v1alpha/backup"
	vars := map[string]interface{}{}
	if projectId != "" {
		vars["project_id"] = projectId
	}
	return SendGet(context, context.IsekaiAddress, path, vars)
}

// DeleteBackup deletes a backup together with all its files. A backup, which is created or
// restored right now, can't be deleted.
func DeleteBackup(context AccessContext, backupUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/backup/%s", backupUuid)
	vars := map[string]interface{}{}
	return SendDelete(context, context.IsekaiAddress, path, vars)
}

// RestoreBackup restores a backup into its own project or into `TargetProjectId`. The response
// contains the plan of the restore and the job, which restores the backup in the background. With
// `DryRun` only the plan is returned, without a job, and nothing is changed.
func RestoreBackup(context AccessContext, backupUuid, passphrase string, options RestoreOptions) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/backup/%s/restore", backupUuid)
	jsonBody := map[string]interface{}{
		"passphrase":           passphrase,
		"force":                options.Force,
		"prune":                options.Prune,
		"start_vms":            options.StartVms,
		"include_dependencies": options.IncludeDependencies,
		"dry_run":              options.DryRun,
	}
	if options.TargetProjectId != "" {
		jsonBody["target_project_id"] = options.TargetProjectId
	}
	return SendPost(context, context.IsekaiAddress, path, jsonBody)
}

// ScanBackupTarget lists the backups on a backup-target, which are not known yet, because they
// were copied onto it from outside, like from another deployment for a disaster-recovery.
// Admin only.
func ScanBackupTarget(context AccessContext, onsenHostUuid string) (map[string]interface{}, error) {
	path := "v1alpha/backup/scan/admin"
	vars := map[string]interface{}{
		"onsen_host_uuid": onsenHostUuid,
	}
	return SendGet(context, context.IsekaiAddress, path, vars)
}

// ImportBackup imports a backup, which was found by ScanBackupTarget, so it can be restored like
// every other backup. The passphrase is checked, before the backup is registered. Without a name,
// the backup is named after its UUID. Admin only.
func ImportBackup(context AccessContext, onsenHostUuid, backupUuid, passphrase, name string) (map[string]interface{}, error) {
	path := "v1alpha/backup/import/admin"
	jsonBody := map[string]interface{}{
		"onsen_host_uuid": onsenHostUuid,
		"backup_uuid":     backupUuid,
		"passphrase":      passphrase,
	}
	if name != "" {
		jsonBody["name"] = name
	}
	return SendPost(context, context.IsekaiAddress, path, jsonBody)
}

func GetJob(context AccessContext, jobUuid string) (map[string]interface{}, error) {
	path := fmt.Sprintf("v1alpha/job/%s", jobUuid)
	vars := map[string]interface{}{}
	return SendGet(context, context.IsekaiAddress, path, vars)
}

// ListJob lists the backup- and restore-jobs. With a backupUuid, only the jobs of this backup are
// listed.
func ListJob(context AccessContext, backupUuid string) (map[string]interface{}, error) {
	path := "v1alpha/job"
	vars := map[string]interface{}{}
	if backupUuid != "" {
		vars["backup_uuid"] = backupUuid
	}
	return SendGet(context, context.IsekaiAddress, path, vars)
}
