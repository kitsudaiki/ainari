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

var getHostCmd = &cobra.Command{
	Use:   "get HOST_UUID",
	Short: "Get information of a specific host.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		hostUuid := args[0]
		content, err := ainari_sdk.GetHost(context, hostUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listHostCmd = &cobra.Command{
	Use:   "list",
	Short: "List all host.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListHost(context)
		if err == nil {
			ainarictl_common.PrintList(content["hosts"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var deleteHostCmd = &cobra.Command{
	Use:   "delete HOST_UUID",
	Short: "Delete a specific host from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		hostUuid := args[0]
		_, err = ainari_sdk.DeleteHost(context, hostUuid)
		if err == nil {
			fmt.Printf("successfully deleted host '%v'\n", hostUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getOnsenHostCmd = &cobra.Command{
	Use:   "get HOST_UUID",
	Short: "Get information of a specific onsen-host.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		hostUuid := args[0]
		content, err := ainari_sdk.GetOnsenHost(context, hostUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listOnsenHostCmd = &cobra.Command{
	Use:   "list",
	Short: "List all onsen-host.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListOnsenHost(context)
		if err == nil {
			ainarictl_common.PrintList(content["hosts"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var deleteOnsenHostCmd = &cobra.Command{
	Use:   "delete HOST_UUID",
	Short: "Delete a specific onsen-host from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		hostUuid := args[0]
		_, err = ainari_sdk.DeleteOnsenHost(context, hostUuid)
		if err == nil {
			fmt.Printf("successfully deleted onsen-host '%v'\n", hostUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var hostCmd = &cobra.Command{
	Use:   "host",
	Short: "Manage the sakura-hosts, which run the virtual machines.",
}

var onsenHostCmd = &cobra.Command{
	Use:   "onsen_host",
	Short: "Manage the onsen-hosts, which store the images and checkpoints.",
}

func Init_Host_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(hostCmd)

	hostCmd.AddCommand(getHostCmd)

	hostCmd.AddCommand(listHostCmd)

	hostCmd.AddCommand(deleteHostCmd)

	rootCmd.AddCommand(onsenHostCmd)

	onsenHostCmd.AddCommand(getOnsenHostCmd)

	onsenHostCmd.AddCommand(listOnsenHostCmd)

	onsenHostCmd.AddCommand(deleteOnsenHostCmd)
}
