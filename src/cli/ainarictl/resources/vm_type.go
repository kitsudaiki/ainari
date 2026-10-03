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
	vmTypeName           string
	vmTypeNumberOfCores  int32
	vmTypeAmountOfMemory int64
)

var createVmTypeCmd = &cobra.Command{
	Use:   "create -c NUMBER_OF_CORES -m AMOUNT_OF_MEMORY NAME",
	Short: "Create a new vm-type.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		name := args[0]
		content, err := ainari_sdk.CreateVmType(context, name, vmTypeNumberOfCores, vmTypeAmountOfMemory)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var updateVmTypeCmd = &cobra.Command{
	Use:   "update [-n NAME] [-c NUMBER_OF_CORES] [-m AMOUNT_OF_MEMORY] VM_TYPE_UUID",
	Short: "Update the values of a specific vm-type. Only the given values are changed.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		vmTypeUuid := args[0]

		// only the flags, which were set, are sent to the backend
		var name *string
		var numberOfCores *int32
		var amountOfMemory *int64
		if cmd.Flags().Changed("name") {
			name = &vmTypeName
		}
		if cmd.Flags().Changed("cores") {
			numberOfCores = &vmTypeNumberOfCores
		}
		if cmd.Flags().Changed("memory") {
			amountOfMemory = &vmTypeAmountOfMemory
		}

		content, err := ainari_sdk.UpdateVmType(context, vmTypeUuid, name, numberOfCores, amountOfMemory)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getVmTypeCmd = &cobra.Command{
	Use:   "get VM_TYPE_UUID",
	Short: "Get information of a specific vm-type.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		vmTypeUuid := args[0]
		content, err := ainari_sdk.GetVmType(context, vmTypeUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listVmTypeCmd = &cobra.Command{
	Use:   "list",
	Short: "List all vm-types.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListVmType(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["vm_types"].([]interface{}))
	},
}

var deleteVmTypeCmd = &cobra.Command{
	Use:   "delete VM_TYPE_UUID",
	Short: "Delete a specific vm-type from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		vmTypeUuid := args[0]
		_, err = ainari_sdk.DeleteVmType(context, vmTypeUuid)
		if err == nil {
			fmt.Printf("successfully deleted vm-type '%v'\n", vmTypeUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var vmTypeCmd = &cobra.Command{
	Use:   "vm_type",
	Short: "Manage the vm-types, which define the cores and memory of virtual machines.",
}

func Init_VmType_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(vmTypeCmd)

	vmTypeCmd.AddCommand(createVmTypeCmd)
	createVmTypeCmd.Flags().Int32VarP(&vmTypeNumberOfCores, "cores", "c", 0, "Number of cpu-cores of the vm-type (mandatory)")
	createVmTypeCmd.Flags().Int64VarP(&vmTypeAmountOfMemory, "memory", "m", 0, "Amount of memory in MiB of the vm-type (mandatory)")
	createVmTypeCmd.MarkFlagRequired("cores")
	createVmTypeCmd.MarkFlagRequired("memory")

	vmTypeCmd.AddCommand(updateVmTypeCmd)
	updateVmTypeCmd.Flags().StringVarP(&vmTypeName, "name", "n", "", "New name of the vm-type")
	updateVmTypeCmd.Flags().Int32VarP(&vmTypeNumberOfCores, "cores", "c", 0, "New number of cpu-cores of the vm-type")
	updateVmTypeCmd.Flags().Int64VarP(&vmTypeAmountOfMemory, "memory", "m", 0, "New amount of memory in MiB of the vm-type")

	vmTypeCmd.AddCommand(getVmTypeCmd)

	vmTypeCmd.AddCommand(listVmTypeCmd)

	vmTypeCmd.AddCommand(deleteVmTypeCmd)
}
