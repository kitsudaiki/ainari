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
	floatingIpName               string
	floatingIpVirtualMachineUuid string
)

var addFloatingIpCmd = &cobra.Command{
	Use:   "add -n NAME [-v VIRTUAL_MACHINE_UUID] [FLOATING_IP]",
	Short: "Reserve a new floating IP. If no FLOATING_IP is given, a free one is selected. If a virtual machine is given, the floating IP is directly attached to it.",
	Args:  cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		floatingIp := ""
		if len(args) == 1 {
			floatingIp = args[0]
		}
		content, err := ainari_sdk.AddFloatingIp(context,
			floatingIpName,
			floatingIp,
			floatingIpVirtualMachineUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var attachFloatingIpCmd = &cobra.Command{
	Use:   "attach FLOATING_IP_UUID VIRTUAL_MACHINE_UUID",
	Short: "Attach a floating IP to a virtual machine.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.AttachFloatingIp(context, args[0], args[1])
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var detachFloatingIpCmd = &cobra.Command{
	Use:   "detach FLOATING_IP_UUID",
	Short: "Detach a floating IP from its virtual machine, so it can be attached to another one.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.DetachFloatingIp(context, args[0])
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getFloatingIpCmd = &cobra.Command{
	Use:   "get FLOATING_IP_UUID",
	Short: "Get information of a specific floating IP.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		floatingIpUuid := args[0]
		content, err := ainari_sdk.GetFloatingIp(context, floatingIpUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listFloatingIpCmd = &cobra.Command{
	Use:   "list",
	Short: "List all floating ip.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListFloatingIp(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["floating_ips"].([]interface{}))
	},
}

var deleteFloatingIpCmd = &cobra.Command{
	Use:   "delete FLOATING_IP_UUID",
	Short: "Delete a specific floating IP from the gateway.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		floatingIpUuid := args[0]
		_, err = ainari_sdk.DeleteFloatingIp(context, floatingIpUuid)
		if err == nil {
			fmt.Printf("successfully deleted floating ip '%v'\n", floatingIpUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var floatingIpCmd = &cobra.Command{
	Use:   "floating_ip",
	Short: "Manage floating ip.",
}

func Init_FloatingIp_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(floatingIpCmd)

	floatingIpCmd.AddCommand(addFloatingIpCmd)
	addFloatingIpCmd.Flags().StringVarP(&floatingIpName, "name", "n", "", "Name of the floating IP (mandatory)")
	addFloatingIpCmd.Flags().StringVarP(&floatingIpVirtualMachineUuid, "virtual_machine", "v", "", "UUID of the virtual machine, which the new floating IP is attached to (optional)")
	addFloatingIpCmd.MarkFlagRequired("name")

	floatingIpCmd.AddCommand(attachFloatingIpCmd)

	floatingIpCmd.AddCommand(detachFloatingIpCmd)

	floatingIpCmd.AddCommand(getFloatingIpCmd)

	floatingIpCmd.AddCommand(listFloatingIpCmd)

	floatingIpCmd.AddCommand(deleteFloatingIpCmd)
}
