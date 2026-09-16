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
	floatingIpName        string
	floatingIpNetworkUuid string
	floatingIpInternalIp  string
)

var addFloatingIpCmd = &cobra.Command{
	Use:   "add -n NAME -u NETWORK_UUID -i INTERNAL_IP FLOATING_IP",
	Short: "Assign a floating IP to an internal IP.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		floatingIp := args[0]
		content, err := ainari_sdk.AddFloatingIp(context,
			floatingIpName,
			floatingIpNetworkUuid,
			floatingIp,
			floatingIpInternalIp)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var deleteFloatingIpCmd = &cobra.Command{
	Use:   "delete FLOATING_IP",
	Short: "Delete a specific floating IP from the gateway.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		floatingIp := args[0]
		_, err = ainari_sdk.DeleteFloatingIp(context, floatingIp)
		if err == nil {
			fmt.Printf("successfully deleted floating ip '%v'\n", floatingIp)
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
	addFloatingIpCmd.Flags().StringVarP(&floatingIpNetworkUuid, "network", "u", "", "UUID of the network, which the internal address belongs to (mandatory)")
	addFloatingIpCmd.Flags().StringVarP(&floatingIpInternalIp, "internal", "i", "", "Internal address, which the floating IP is assigned to (mandatory)")
	addFloatingIpCmd.MarkFlagRequired("name")
	addFloatingIpCmd.MarkFlagRequired("network")
	addFloatingIpCmd.MarkFlagRequired("internal")

	floatingIpCmd.AddCommand(deleteFloatingIpCmd)
}
