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
	virtual_machineVmTypeUuid    string
	virtual_machineDiskSize      int64
	virtual_machineNetworkUuid   string
	virtual_machineImageUuid     string
	virtual_machinePublicKeyUuid string
	snapshotName               string
	virtual_machineMode          string
	virtual_machineIsolatedHost  bool
	virtual_machineHostUuid      string
	virtual_machineTargetHost    string
)

var createVirtualMachineCmd = &cobra.Command{
	Use:   "create -t VM_TYPE_UUID -d DISK_SIZE -u NETWORK_UUID -i IMAGE_UUID -k PUBLIC_KEY_UUID [--isolated_host] [--host HOST_UUID] NAME",
	Short: "Create a new virtual machine.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineName := args[0]

		// reserve the virtual machine on one of the sakura-hosts
		content, err := ainari_sdk.ReserveVirtualMachine(context,
			virtual_machineName,
			virtual_machineVmTypeUuid,
			virtual_machineDiskSize,
			virtual_machineNetworkUuid,
			virtual_machineIsolatedHost,
			virtual_machineHostUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}

		virtual_machineUuid, toriiAddress := getVirtualMachineAccess(content)

		// install image and public-key in the reserved virtual machine and boot it
		_, err = ainari_sdk.CreateVirtualMachine(context,
			toriiAddress,
			virtual_machineUuid,
			virtual_machineImageUuid,
			virtual_machinePublicKeyUuid)
		if err != nil {
			fmt.Println(err)
			fmt.Printf("virtual machine '%v' is still reserved and has to be deleted manually\n", virtual_machineUuid)
			os.Exit(1)
		}

		// the output of the reservation doesn't contain the image yet, so the virtual machine is
		// requested again after the creation was started
		content, err = ainari_sdk.GetVirtualMachine(context, virtual_machineUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

// getVirtualMachineAccess reads the uuid and the torii-address of a virtual machine from the
// output of the reserve-call, which are required to address the sakura-host of the virtual machine.
func getVirtualMachineAccess(virtual_machine_data map[string]interface{}) (string, string) {
	value, ok := virtual_machine_data["uuid"]
	if !ok {
		fmt.Println("key 'uuid' not found in virtual_machine-output")
		os.Exit(1)
	}
	virtual_machineUuid, ok := value.(string)
	if !ok {
		fmt.Println("uuid is not a string")
		os.Exit(1)
	}

	value, ok = virtual_machine_data["torii_address"]
	if !ok {
		fmt.Println("key 'torii_address' not found in virtual_machine-output")
		os.Exit(1)
	}
	toriiAddress, ok := value.(string)
	if !ok {
		fmt.Println("torii_address is not a string")
		os.Exit(1)
	}

	return virtual_machineUuid, toriiAddress
}

var getVirtualMachineCmd = &cobra.Command{
	Use:   "get VIRTUAL_MACHINE_UUID",
	Short: "Get information of a specific virtual machine.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		content, err := ainari_sdk.GetVirtualMachine(context, virtual_machineUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var listVirtualMachineCmd = &cobra.Command{
	Use:   "list",
	Short: "List all virtual machine.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListVirtualMachine(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["virtual_machines"].([]interface{}))
	},
}

var deleteVirtualMachineCmd = &cobra.Command{
	Use:   "delete VIRTUAL_MACHINE_UUID",
	Short: "Delete a specific virtual machine from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		_, err = ainari_sdk.DeleteVirtualMachine(context, virtual_machineUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		fmt.Printf("deletion of virtual machine '%v' started\n", virtual_machineUuid)
	},
}

// newVirtualMachinePowerCmd builds a command, which creates a task on the sakura-host of a virtual
// machine to change its power-state. The start-, stop- and reboot-commands only differ in the
// called sdk-function.
func newVirtualMachinePowerCmd(
	use string,
	short string,
	powerFunc func(ainari_sdk.AccessContext, string, string) (map[string]interface{}, error),
) *cobra.Command {
	return &cobra.Command{
		Use:   use + " VIRTUAL_MACHINE_UUID",
		Short: short,
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			context, err := Login()
			if err != nil {
				fmt.Println(err)
				os.Exit(1)
			}
			virtual_machineUuid := args[0]
			toriiAddress := getToriiAddress(context, virtual_machineUuid)
			content, err := powerFunc(context, toriiAddress, virtual_machineUuid)
			if err != nil {
				fmt.Println(err)
				os.Exit(1)
			}
			ainarictl_common.PrintSingle(content)
		},
	}
}

var startVirtualMachineCmd = newVirtualMachinePowerCmd(
	"start",
	"Create a new task to boot a stopped virtual machine again.",
	ainari_sdk.StartVirtualMachine)

var stopVirtualMachineCmd = newVirtualMachinePowerCmd(
	"stop",
	"Create a new task to shut down a virtual machine, which keeps all of its resources.",
	ainari_sdk.StopVirtualMachine)

var rebootVirtualMachineCmd = newVirtualMachinePowerCmd(
	"reboot",
	"Create a new task to reboot a running virtual machine.",
	ainari_sdk.RebootVirtualMachine)

var migrateVirtualMachineCmd = &cobra.Command{
	Use:   "migrate -t TARGET_HOST_UUID VIRTUAL_MACHINE_UUID",
	Short: "Move a virtual machine to another sakura-host (admin only).",
	Long: `Move a virtual machine to another sakura-host. This can only be done by an admin.

The virtual machine is shut down, its disk is transferred to the target host and it is booted
there again, if it was running before. It keeps its address, its proxy-address and its
packet-filters. The migration runs in the background: the virtual machine is in the state
MIGRATING, until it is running or stopped on the target host again. If the migration fails, the
virtual machine is started on its current host again.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		virtual_machineUuid := args[0]
		content, err := ainari_sdk.MigrateVirtualMachine(context, virtual_machineUuid, virtual_machineTargetHost)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var getVirtualMachineCountCmd = &cobra.Command{
	Use:   "count",
	Short: "Get the number of virtual machines of the project.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.GetVirtualMachineCount(context)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var virtual_machineCmd = &cobra.Command{
	Use:   "vm",
	Short: "Manage virtual machines.",
}

func Init_VirtualMachine_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(virtual_machineCmd)

	virtual_machineCmd.AddCommand(createVirtualMachineCmd)
	createVirtualMachineCmd.Flags().StringVarP(&virtual_machineVmTypeUuid, "vm_type", "t", "", "UUID of the vm-type, which defines the cores and memory of the virtual machine (mandatory)")
	createVirtualMachineCmd.Flags().Int64VarP(&virtual_machineDiskSize, "disk", "d", 0, "Size of the disk in GiB of the virtual machine (mandatory)")
	createVirtualMachineCmd.Flags().StringVarP(&virtual_machineNetworkUuid, "network", "u", "", "UUID of the network of the virtual machine (mandatory)")
	createVirtualMachineCmd.Flags().StringVarP(&virtual_machineImageUuid, "image", "i", "", "UUID of the image of the virtual machine (mandatory)")
	createVirtualMachineCmd.Flags().StringVarP(&virtual_machinePublicKeyUuid, "public_key", "k", "", "UUID of the public-key, which is deployed in the virtual machine (mandatory)")
	createVirtualMachineCmd.Flags().BoolVar(&virtual_machineIsolatedHost, "isolated_host", false, "Place the virtual machine only on a host, which is isolated for the project")
	createVirtualMachineCmd.Flags().StringVar(&virtual_machineHostUuid, "host", "", "UUID of the sakura-host, which runs the virtual machine (admin only, fails without enough free resources on it)")
	createVirtualMachineCmd.MarkFlagRequired("vm_type")
	createVirtualMachineCmd.MarkFlagRequired("disk")
	createVirtualMachineCmd.MarkFlagRequired("network")
	createVirtualMachineCmd.MarkFlagRequired("image")
	createVirtualMachineCmd.MarkFlagRequired("public_key")

	virtual_machineCmd.AddCommand(getVirtualMachineCmd)

	virtual_machineCmd.AddCommand(listVirtualMachineCmd)

	virtual_machineCmd.AddCommand(deleteVirtualMachineCmd)

	virtual_machineCmd.AddCommand(startVirtualMachineCmd)

	virtual_machineCmd.AddCommand(stopVirtualMachineCmd)

	virtual_machineCmd.AddCommand(rebootVirtualMachineCmd)

	virtual_machineCmd.AddCommand(getVirtualMachineCountCmd)

	virtual_machineCmd.AddCommand(migrateVirtualMachineCmd)
	migrateVirtualMachineCmd.Flags().StringVarP(&virtual_machineTargetHost, "target", "t", "", "UUID of the sakura-host, which runs the virtual machine afterwards (mandatory)")
	migrateVirtualMachineCmd.MarkFlagRequired("target")
}
