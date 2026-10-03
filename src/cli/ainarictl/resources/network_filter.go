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

// changeNetworkFilterCommand builds a command, which changes one include-list of the packet
// filter of a virtual machine. All of them take the virtual machine, the direction and the
// entries to add or remove.
func changeNetworkFilterCommand(use, entryName, short string,
	change func(ainari_sdk.AccessContext, string, string, []string) (map[string]interface{}, error)) *cobra.Command {
	return &cobra.Command{
		Use:   use + " VIRTUAL_MACHINE_UUID DIRECTION " + entryName + " [" + entryName + " ...]",
		Short: short,
		Args:  cobra.MinimumNArgs(3),
		Run: func(cmd *cobra.Command, args []string) {
			context, err := Login()
			if err != nil {
				fmt.Println(err)
				os.Exit(1)
			}
			content, err := change(context, args[0], args[1], args[2:])
			if err == nil {
				ainarictl_common.PrintSingle(content)
			} else {
				fmt.Println(err)
				os.Exit(1)
			}
		},
	}
}

var addNetworkFilterIpRangeCmd = changeNetworkFilterCommand("add_ip_range", "IP_RANGE",
	"Allow ip-ranges in the packet filter of one direction (ingress or egress) of a virtual machine. An ip-range is a single address, a subnet in CIDR notation or a range FIRST-LAST.",
	ainari_sdk.AddNetworkFilterIpRanges)

var removeNetworkFilterIpRangeCmd = changeNetworkFilterCommand("remove_ip_range", "IP_RANGE",
	"Remove ip-ranges from the packet filter of one direction (ingress or egress) of a virtual machine.",
	ainari_sdk.DeleteNetworkFilterIpRanges)

var addNetworkFilterPortCmd = changeNetworkFilterCommand("add_port", "PORT",
	"Allow ports in the packet filter of one direction (ingress or egress) of a virtual machine. A port is a single port or a range FIRST-LAST.",
	ainari_sdk.AddNetworkFilterPorts)

var removeNetworkFilterPortCmd = changeNetworkFilterCommand("remove_port", "PORT",
	"Remove ports from the packet filter of one direction (ingress or egress) of a virtual machine.",
	ainari_sdk.DeleteNetworkFilterPorts)

var getNetworkFilterCmd = &cobra.Command{
	Use:   "get VIRTUAL_MACHINE_UUID DIRECTION",
	Short: "Get the packet filter of one direction (ingress or egress) of a virtual machine.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.GetNetworkFilter(context, args[0], args[1])
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listNetworkFilterCmd = &cobra.Command{
	Use:   "list",
	Short: "List all packet filters of the virtual machines.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListNetworkFilter(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["network_filters"].([]interface{}))
	},
}

var deleteNetworkFilterCmd = &cobra.Command{
	Use:   "delete VIRTUAL_MACHINE_UUID DIRECTION",
	Short: "Delete the packet filter of one direction (ingress or egress) of a virtual machine, which allows all traffic of this direction again.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		_, err = ainari_sdk.DeleteNetworkFilter(context, args[0], args[1])
		if err == nil {
			fmt.Printf("successfully deleted %v network filter of virtual machine '%v'\n", args[1], args[0])
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var networkFilterCmd = &cobra.Command{
	Use:   "network_filter",
	Short: "Manage the packet filters of virtual machines.",
}

func Init_NetworkFilter_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(networkFilterCmd)

	networkFilterCmd.AddCommand(addNetworkFilterIpRangeCmd)

	networkFilterCmd.AddCommand(removeNetworkFilterIpRangeCmd)

	networkFilterCmd.AddCommand(addNetworkFilterPortCmd)

	networkFilterCmd.AddCommand(removeNetworkFilterPortCmd)

	networkFilterCmd.AddCommand(getNetworkFilterCmd)

	networkFilterCmd.AddCommand(listNetworkFilterCmd)

	networkFilterCmd.AddCommand(deleteNetworkFilterCmd)
}
