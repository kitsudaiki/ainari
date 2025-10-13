/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
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

var hostHeader = []string{
	"uuid",
	"type",
}

var listHostsCmd = &cobra.Command{
	Use:   "list",
	Short: "List all logical hosts.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListHosts(context)
		if err == nil {
			ainarictl_common.PrintList(content["hosts"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var hostsCmd = &cobra.Command{
	Use:   "host",
	Short: "Manage hosts.",
}

func Init_Host_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(hostsCmd)

	hostsCmd.AddCommand(listHostsCmd)
}
