/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>
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

var getProxyCmd = &cobra.Command{
	Use:   "get PROXY_UUID",
	Short: "Get information of a specific proxy.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		proxyUuid := args[0]
		content, err := ainari_sdk.GetProxy(context, proxyUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listProxyCmd = &cobra.Command{
	Use:   "list",
	Short: "List all proxy.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListProxy(context)
		if err == nil {
			ainarictl_common.PrintList(content["proxys"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var deleteProxyCmd = &cobra.Command{
	Use:   "delete PROXY_UUID",
	Short: "Delete a specific proxy from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		proxyUuid := args[0]
		_, err = ainari_sdk.DeleteProxy(context, proxyUuid)
		if err == nil {
			fmt.Printf("successfully deleted proxy '%v'\n", proxyUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var proxyCmd = &cobra.Command{
	Use:   "proxy",
	Short: "Manage proxy.",
}

func Init_Proxy_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(proxyCmd)
	
	proxyCmd.AddCommand(getProxyCmd)

	proxyCmd.AddCommand(listProxyCmd)

	proxyCmd.AddCommand(deleteProxyCmd)
}
