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
	"fmt"
	ainarictl_common "ainarictl/common"
	"os"

	ainari_sdk "github.com/kitsudaiki/ainari"
	"github.com/spf13/cobra"
)

var (
	snapshotImageUuid string
)

func getToriiPort(context ainari_sdk.AccessContext, virtual_machineUuid string) int {

	virtual_machine_data, err := ainari_sdk.GetVirtualMachine(context, virtual_machineUuid)
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}

	value, ok := virtual_machine_data["torii_port"]
	if !ok {
		fmt.Println("key 'torii_port' not found in virtual_machine-output")
		os.Exit(1)
	}

	toriiPort, ok := value.(float64) // Golang is stupid! Why beomes a 'u16' and 'float64' ?!?!?!
	if !ok {
		fmt.Println("toriiPort is not an int")
		os.Exit(1)
	}

	return int(toriiPort)
}

var createSnapshotSaveTaskCmd = &cobra.Command{
	Use:   "snapshot_create VIRTUAL_MACHINE_UUID SNAPSHOT_NAME",
	Short: "Create a new task to save the root-disk of a virtual_machine as new snapshot-image.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		toriiPort := getToriiPort(context, virtual_machineUuid)
		snapshotName := args[1]
		content, err := ainari_sdk.CreateSnapshotSaveTask(context, toriiPort, snapshotName, virtual_machineUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createSnapshotRestoreTaskCmd = &cobra.Command{
	Use:   "snapshot_restore -i IMAGE_UUID VIRTUAL_MACHINE_UUID TASK_NAME",
	Short: "Create a new task to reset the root-disk of a virtual_machine to a snapshot.",
	Long: `Create a new task to reset the root-disk of a virtual_machine to a snapshot.

Only images, which are marked as snapshot, can be restored. The virtual_machine is shut down,
while its root-disk is replaced, and booted again afterwards.`,
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		toriiPort := getToriiPort(context, virtual_machineUuid)
		taskName := args[1]
		content, err := ainari_sdk.CreateSnapshotRestoreTask(context, toriiPort, taskName, virtual_machineUuid, snapshotImageUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getTaskCmd = &cobra.Command{
	// the task itself is not bound to a virtual machine anymore, but the virtual machine is still
	// required here to address the sakura-host, which holds the task
	Use:   "get VIRTUAL_MACHINE_UUID TASK_UUID",
	Short: "Get information of a specific task of the sakura-host of a virtual machine.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		toriiPort := getToriiPort(context, virtual_machineUuid)
		taskUuid := args[1]
		content, err := ainari_sdk.GetTask(context, toriiPort, taskUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listTaskCmd = &cobra.Command{
	// the tasks are not listed per virtual machine anymore, but the virtual machine is still
	// required here to address the sakura-host, whose tasks are listed
	Use:   "list VIRTUAL_MACHINE_UUID",
	Short: "List all tasks of the sakura-host of a virtual machine.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		toriiPort := getToriiPort(context, virtual_machineUuid)
		content, err := ainari_sdk.ListTask(context, toriiPort)
		if err == nil {
			ainarictl_common.PrintList(content["tasks"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var abortTaskCmd = &cobra.Command{
	// the task itself is not bound to a virtual machine anymore, but the virtual machine is still
	// required here to address the sakura-host, which holds the task
	Use:   "abort VIRTUAL_MACHINE_UUID TASK_UUID",
	Short: "Abort a specific task of the sakura-host of a virtual machine.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		toriiPort := getToriiPort(context, virtual_machineUuid)
		taskUuid := args[1]
		content, err := ainari_sdk.AbortTask(context, toriiPort, taskUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var taskCmd = &cobra.Command{
	Use:   "task",
	Short: "Manage task.",
}

var createTaskCmd = &cobra.Command{
	Use:   "create",
	Short: "Create new task.",
}

func Init_Task_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(taskCmd)

	taskCmd.AddCommand(createTaskCmd)

	createTaskCmd.AddCommand(createSnapshotSaveTaskCmd)

	createTaskCmd.AddCommand(createSnapshotRestoreTaskCmd)
	createSnapshotRestoreTaskCmd.Flags().StringVarP(&snapshotImageUuid, "image_uuid", "i", "", "UUID of the image, which must be a snapshot (mandatory)")
	createSnapshotRestoreTaskCmd.MarkFlagRequired("image_uuid")

	taskCmd.AddCommand(getTaskCmd)

	taskCmd.AddCommand(listTaskCmd)

	taskCmd.AddCommand(abortTaskCmd)
}
