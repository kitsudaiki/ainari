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
	networkSubnet string
)

var createNetworkCmd = &cobra.Command{
	Use:   "create -s SUBNET NAME",
	Short: "Create a new network.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		networkName := args[0]
		content, err := ainari_sdk.CreateNetwork(context, networkName, networkSubnet)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getNetworkCmd = &cobra.Command{
	Use:   "get NETWORK_UUID",
	Short: "Get information of a specific network.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		networkUuid := args[0]
		content, err := ainari_sdk.GetNetwork(context, networkUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listNetworkCmd = &cobra.Command{
	Use:   "list",
	Short: "List all network.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListNetwork(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["networks"].([]interface{}))
	},
}

var deleteNetworkCmd = &cobra.Command{
	Use:   "delete NETWORK_UUID",
	Short: "Delete a specific network from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		networkUuid := args[0]
		_, err = ainari_sdk.DeleteNetwork(context, networkUuid)
		if err == nil {
			fmt.Printf("successfully deleted network '%v'\n", networkUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var networkCmd = &cobra.Command{
	Use:   "network",
	Short: "Manage network.",
}

func Init_Network_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(networkCmd)

	networkCmd.AddCommand(createNetworkCmd)
	createNetworkCmd.Flags().StringVarP(&networkSubnet, "subnet", "s", "", "Subnet of the network in CIDR-notation (mandatory)")
	createNetworkCmd.MarkFlagRequired("subnet")

	networkCmd.AddCommand(getNetworkCmd)

	networkCmd.AddCommand(listNetworkCmd)

	networkCmd.AddCommand(deleteNetworkCmd)
}
