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

package ainari_resources

import (
	ainarictl_common "ainarictl/common"
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"

	ainari_sdk "github.com/kitsudaiki/ainari"
	"github.com/spf13/cobra"
)

// env-variable, which contains the passphrase of a backup, so it doesn't have to be typed in
const backupPassphraseEnv = "AINARI_BACKUP_PASSPHRASE"

var (
	backupPassphraseFile string
	backupStopVms        bool
	backupWithoutImages  bool
	backupWait           bool
	backupProjectId      string
	backupJobBackupUuid  string
	backupImportName     string
	restoreOptions       ainari_sdk.RestoreOptions
	restoreNoDependency  bool
)

// readBackupPassphrase returns the passphrase of a backup. It is never given as argument, because
// it would be visible in the history of the shell. It is read from the file of
// '--passphrase_file', from the env-variable AINARI_BACKUP_PASSPHRASE or from the terminal. A new
// passphrase is requested two times from the terminal.
func readBackupPassphrase(isNew bool) string {
	if backupPassphraseFile != "" {
		content, err := os.ReadFile(backupPassphraseFile)
		if err != nil {
			fmt.Printf("Failed to read the passphrase from '%v': %v\n", backupPassphraseFile, err)
			os.Exit(1)
		}
		return strings.TrimSpace(string(content))
	}
	if value := os.Getenv(backupPassphraseEnv); value != "" {
		return value
	}
	if !isNew {
		return readPassphrase("Enter passphrase of the backup: ")
	}

	passphrase1 := readPassphrase("Enter passphrase of the backup: ")
	passphrase2 := readPassphrase("Enter passphrase of the backup again: ")
	if passphrase1 != passphrase2 {
		fmt.Println("Mismatch between the two entered passphrases")
		os.Exit(1)
	}
	return passphrase1
}

// exitOnError prints the error and stops the cli.
func exitOnError(err error) {
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

// waitForJob polls a job of isekai, until it is done, and returns it.
func waitForJob(context ainari_sdk.AccessContext, jobUuid string) map[string]interface{} {
	lastStep := ""
	for {
		job, err := ainari_sdk.GetJob(context, jobUuid)
		exitOnError(err)
		if step, _ := job["step"].(string); step != lastStep && !ainarictl_common.PrintAsJson {
			fmt.Fprintf(os.Stderr, "%v\n", step)
			lastStep = step
		}
		if state, _ := job["state"].(string); state == "FINISHED" || state == "ERROR" {
			return job
		}
		time.Sleep(2 * time.Second)
	}
}

// printJob prints a job of isekai. The resources, which got a new UUID by a restore, are printed
// as separate table.
func printJob(job map[string]interface{}) {
	if ainarictl_common.PrintAsJson {
		ainarictl_common.PrintSingle(job)
		return
	}
	idMap, _ := job["id_map"].([]interface{})
	summary := map[string]interface{}{}
	for key, value := range job {
		if key == "id_map" {
			continue
		}
		if list, ok := value.([]interface{}); ok {
			value = joinValues(list)
		}
		summary[key] = value
	}
	ainarictl_common.PrintSingle(summary)
	if len(idMap) > 0 {
		fmt.Println("resources with a new UUID:")
		ainarictl_common.PrintList(idMap)
	}
}

// joinValues converts a list of values into one line per value.
func joinValues(list []interface{}) string {
	lines := make([]string, 0, len(list))
	for _, value := range list {
		lines = append(lines, fmt.Sprintf("%v", value))
	}
	return strings.Join(lines, "\n")
}

// printBackupJob prints the response of a new backup or a restore. With '--wait' the job is
// awaited and the finished job is printed, which fails the cli, if the job failed.
func printBackupJob(context ainari_sdk.AccessContext, content map[string]interface{}, key string) {
	job, _ := content["job"].(map[string]interface{})
	if job != nil && backupWait {
		job = waitForJob(context, job["uuid"].(string))
		content["job"] = job
		// the state and the size of a new backup are only known, after its job is done
		if data, ok := content[key].(map[string]interface{}); ok {
			if backup, err := ainari_sdk.GetBackup(context, data["uuid"].(string)); err == nil {
				content[key] = backup
			}
		}
	}

	if ainarictl_common.PrintAsJson {
		ainarictl_common.PrintSingle(content)
	} else {
		if data, ok := content[key].(map[string]interface{}); ok {
			ainarictl_common.PrintSingle(data)
		}
		if job != nil {
			fmt.Println("job:")
			printJob(job)
		}
	}
	if job != nil && job["state"] == "ERROR" {
		os.Exit(1)
	}
}

// printRestorePlan prints the plan of a restore section by section.
func printRestorePlan(plan map[string]interface{}) {
	fmt.Printf("target project: %v", plan["target_project_id"])
	if create, _ := plan["create_project"].(bool); create {
		fmt.Print(" (created by the restore)")
	}
	fmt.Println()

	for _, section := range []struct{ key, title string }{
		{"id_map", "resources with a new UUID:"},
		{"overwritten", "overwritten resources:"},
		{"deleted", "deleted resources:"},
		{"quota", "quota after the restore:"},
	} {
		if list, _ := plan[section.key].([]interface{}); len(list) > 0 {
			fmt.Println(section.title)
			ainarictl_common.PrintList(list)
		}
	}
	if warnings, _ := plan["warnings"].([]interface{}); len(warnings) > 0 {
		fmt.Println("warnings:")
		for _, warning := range warnings {
			fmt.Printf("    %v\n", warning)
		}
	}
}

var createProjectBackupCmd = &cobra.Command{
	Use:   "create_project BACKUP_NAME",
	Short: "Create a backup of the whole project of the current token.",
	Long: `Create a backup of the whole project of the current token.

The backup contains the project with its quota, the secrets and public-keys, the images and
snapshots, the networks, virtual_machines, packet-filters and floating ip-addresses and a copy of
the root-disk of every virtual_machine. Users and their assignments are not part of it. The
project is locked, until the backup is finished, so its resources can only be read meanwhile.

The backup is encrypted with a key, which is derived from its passphrase. The passphrase is never
stored, so the backup can't be restored without it. It is read from the file of
'--passphrase_file', from the env-variable AINARI_BACKUP_PASSPHRASE or from the terminal.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		passphrase := readBackupPassphrase(true)
		content, err := ainari_sdk.CreateProjectBackup(context, args[0], passphrase, backupStopVms, !backupWithoutImages)
		exitOnError(err)
		printBackupJob(context, content, "backup")
	},
}

var createVirtualMachineBackupCmd = &cobra.Command{
	Use:   "create_vm VIRTUAL_MACHINE_UUID BACKUP_NAME",
	Short: "Create a backup of a single virtual_machine.",
	Long: `Create a backup of a single virtual_machine.

The backup contains the virtual_machine with a copy of its root-disk, its public-key, its network
and its packet-filters. The project is locked, until the backup is finished.

The passphrase is read from the file of '--passphrase_file', from the env-variable
AINARI_BACKUP_PASSPHRASE or from the terminal.`,
	Args: cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		passphrase := readBackupPassphrase(true)
		content, err := ainari_sdk.CreateVirtualMachineBackup(context, args[0], args[1], passphrase, backupStopVms)
		exitOnError(err)
		printBackupJob(context, content, "backup")
	},
}

var getBackupCmd = &cobra.Command{
	Use:   "get BACKUP_UUID",
	Short: "Get information of a specific backup.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		content, err := ainari_sdk.GetBackup(context, args[0])
		exitOnError(err)
		ainarictl_common.PrintSingle(content)
	},
}

var listBackupCmd = &cobra.Command{
	Use:   "list",
	Short: "List the backups of the project of the current token. Admins see all backups.",
	Args:  cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		content, err := ainari_sdk.ListBackup(context, backupProjectId)
		exitOnError(err)
		ainarictl_common.PrintList(content["backups"].([]interface{}))
	},
}

var deleteBackupCmd = &cobra.Command{
	Use:   "delete BACKUP_UUID",
	Short: "Delete a specific backup together with all its files.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		_, err = ainari_sdk.DeleteBackup(context, args[0])
		exitOnError(err)
		fmt.Printf("successfully deleted backup '%v'\n", args[0])
	},
}

var restoreBackupCmd = &cobra.Command{
	Use:   "restore BACKUP_UUID",
	Short: "Restore a backup into its project or into another one.",
	Long: `Restore a backup into its project or into another one.

Without '--force', a resource, whose UUID is already used, gets a new UUID. With '--force', the
existing resources with the same UUID are overwritten. '--prune', which is only allowed together
with '--force', deletes the resources of the project, which are not part of the backup. With
'--dry_run' only the plan of the restore is printed and nothing is changed.

The passphrase is read from the file of '--passphrase_file', from the env-variable
AINARI_BACKUP_PASSPHRASE or from the terminal.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		passphrase := readBackupPassphrase(false)
		options := restoreOptions
		options.IncludeDependencies = !restoreNoDependency
		content, err := ainari_sdk.RestoreBackup(context, args[0], passphrase, options)
		exitOnError(err)

		plan, _ := content["plan"].(map[string]interface{})
		if !ainarictl_common.PrintAsJson && plan != nil {
			printRestorePlan(plan)
		}
		if job, _ := content["job"].(map[string]interface{}); job != nil {
			printBackupJob(context, content, "")
		} else if ainarictl_common.PrintAsJson {
			ainarictl_common.PrintSingle(content)
		}
	},
}

var scanBackupTargetCmd = &cobra.Command{
	Use:   "scan ONSEN_HOST_UUID",
	Short: "List the backups on a backup-target, which are not known yet (admin only).",
	Long: `List the backups on a backup-target, which are not known yet (admin only).

These are backups, which were copied onto the backup-target from outside, for example from another
deployment for a disaster-recovery. They can be imported with 'backup import'.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		content, err := ainari_sdk.ScanBackupTarget(context, args[0])
		exitOnError(err)
		ainarictl_common.PrintList(content["backups"].([]interface{}))
	},
}

var importBackupCmd = &cobra.Command{
	Use:   "import ONSEN_HOST_UUID BACKUP_UUID",
	Short: "Import a backup, which was found by 'backup scan' (admin only).",
	Long: `Import a backup, which was found by 'backup scan' (admin only).

The passphrase is checked, before the backup is registered. Afterwards it can be restored like
every other backup. The passphrase is read from the file of '--passphrase_file', from the
env-variable AINARI_BACKUP_PASSPHRASE or from the terminal.`,
	Args: cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		passphrase := readBackupPassphrase(false)
		content, err := ainari_sdk.ImportBackup(context, args[0], args[1], passphrase, backupImportName)
		exitOnError(err)
		ainarictl_common.PrintSingle(content)
	},
}

var getBackupJobCmd = &cobra.Command{
	Use:   "job_get JOB_UUID",
	Short: "Get a specific backup- or restore-job.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		content, err := ainari_sdk.GetJob(context, args[0])
		exitOnError(err)
		printJob(content)
	},
}

var listBackupJobCmd = &cobra.Command{
	Use:   "job_list",
	Short: "List the backup- and restore-jobs.",
	Args:  cobra.NoArgs,
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		content, err := ainari_sdk.ListJob(context, backupJobBackupUuid)
		exitOnError(err)
		jobs := content["jobs"].([]interface{})
		if !ainarictl_common.PrintAsJson {
			// the messages and the id-map don't fit into the table, they are shown by 'job_get'
			for _, job := range jobs {
				delete(job.(map[string]interface{}), "messages")
				delete(job.(map[string]interface{}), "id_map")
			}
		}
		ainarictl_common.PrintList(jobs)
	},
}

var setBackupTargetCmd = &cobra.Command{
	Use:   "set_backup_target HOST_UUID true|false",
	Short: "Set, if a specific onsen-host is a backup-target (admin only).",
	Long: `Set, if a specific onsen-host is a backup-target (admin only).

A backup-target only stores the backups of isekai and never normal images or snapshots. A host
can only be marked, while no images or snapshots are stored on it, and only be unmarked, while no
backups are stored on it.`,
	Args: cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		isBackupTarget, err := strconv.ParseBool(args[1])
		if err != nil {
			fmt.Printf("invalid value '%v' for the backup-target, expected true or false\n", args[1])
			os.Exit(1)
		}
		content, err := ainari_sdk.SetOnsenHostBackupTarget(context, args[0], isBackupTarget)
		exitOnError(err)
		ainarictl_common.PrintSingle(content)
	},
}

var lockProjectCmd = &cobra.Command{
	Use:   "lock PROJECT_ID REASON",
	Short: "Lock a project, so its resources can only be read (admin only).",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		content, err := ainari_sdk.LockProject(context, args[0], args[1])
		exitOnError(err)
		ainarictl_common.PrintSingle(content)
	},
}

var unlockProjectCmd = &cobra.Command{
	Use:   "unlock PROJECT_ID",
	Short: "Remove the lock of a project, also the one of a failed restore (admin only).",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		exitOnError(err)
		_, err = ainari_sdk.UnlockProject(context, args[0])
		exitOnError(err)
		fmt.Printf("successfully unlocked project '%v'\n", args[0])
	},
}

var backupCmd = &cobra.Command{
	Use:   "backup",
	Short: "Manage the backups of virtual_machines and projects and restore them.",
}

func Init_Backup_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(backupCmd)

	passphraseFileText := "File, which contains the passphrase of the backup. Without it, the " +
		"passphrase is read from the env-variable " + backupPassphraseEnv + " or from the terminal."
	waitText := "Wait until the job is finished and print it. Fails, if the job failed."

	backupCmd.AddCommand(createProjectBackupCmd)
	createProjectBackupCmd.Flags().StringVar(&backupPassphraseFile, "passphrase_file", "", passphraseFileText)
	createProjectBackupCmd.Flags().BoolVar(&backupStopVms, "stop_vms", false,
		"Shut the running virtual_machines down, while their root-disks are saved")
	createProjectBackupCmd.Flags().BoolVar(&backupWithoutImages, "without_images", false,
		"Don't copy the images and snapshots of the project into the backup")
	createProjectBackupCmd.Flags().BoolVarP(&backupWait, "wait", "w", false, waitText)

	backupCmd.AddCommand(createVirtualMachineBackupCmd)
	createVirtualMachineBackupCmd.Flags().StringVar(&backupPassphraseFile, "passphrase_file", "", passphraseFileText)
	createVirtualMachineBackupCmd.Flags().BoolVar(&backupStopVms, "stop_vm", false,
		"Shut the virtual_machine down, while its root-disk is saved")
	createVirtualMachineBackupCmd.Flags().BoolVarP(&backupWait, "wait", "w", false, waitText)

	backupCmd.AddCommand(getBackupCmd)

	backupCmd.AddCommand(listBackupCmd)
	listBackupCmd.Flags().StringVar(&backupProjectId, "project", "",
		"List only the backups of this project (admin only)")

	backupCmd.AddCommand(deleteBackupCmd)

	backupCmd.AddCommand(restoreBackupCmd)
	restoreBackupCmd.Flags().StringVar(&backupPassphraseFile, "passphrase_file", "", passphraseFileText)
	restoreBackupCmd.Flags().BoolVar(&restoreOptions.Force, "force", false,
		"Overwrite the existing resources with the same UUID")
	restoreBackupCmd.Flags().BoolVar(&restoreOptions.Prune, "prune", false,
		"Delete the resources of the project, which are not part of the backup (requires --force)")
	restoreBackupCmd.Flags().StringVar(&restoreOptions.TargetProjectId, "target_project", "",
		"Restore into this project instead of the project of the backup")
	restoreBackupCmd.Flags().BoolVar(&restoreOptions.StartVms, "start_vms", false,
		"Start all restored virtual_machines, instead of restoring their power-state")
	restoreBackupCmd.Flags().BoolVar(&restoreNoDependency, "without_dependencies", false,
		"Don't create the missing network of a virtual_machine from the backup")
	restoreBackupCmd.Flags().BoolVar(&restoreOptions.DryRun, "dry_run", false,
		"Only print the plan of the restore without changing anything")
	restoreBackupCmd.Flags().BoolVarP(&backupWait, "wait", "w", false, waitText)

	backupCmd.AddCommand(scanBackupTargetCmd)

	backupCmd.AddCommand(importBackupCmd)
	importBackupCmd.Flags().StringVar(&backupPassphraseFile, "passphrase_file", "", passphraseFileText)
	importBackupCmd.Flags().StringVarP(&backupImportName, "name", "n", "",
		"Name of the imported backup, its UUID by default")

	backupCmd.AddCommand(getBackupJobCmd)

	backupCmd.AddCommand(listBackupJobCmd)
	listBackupJobCmd.Flags().StringVar(&backupJobBackupUuid, "backup", "",
		"List only the jobs of this backup")

	onsenHostCmd.AddCommand(setBackupTargetCmd)

	projectCmd.AddCommand(lockProjectCmd)
	projectCmd.AddCommand(unlockProjectCmd)
}
