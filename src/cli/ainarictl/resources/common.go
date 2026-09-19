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

var getVersionCmd = &cobra.Command{
	Use:   "get version",
	Short: "Get version of the backend.",
	Run: func(cmd *cobra.Command, args []string) {
		var context ainari_sdk.AccessContext
		var err error
		context, err = Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.GetVersion(context, context.MikoAddress)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}


var getEndpointsCmd = &cobra.Command{
	Use:   "endpoints",
	Short: "Get the addresses of the components of the backend.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.GetEndpoints(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var renewTokenCmd = &cobra.Command{
	Use:   "renew",
	Short: "Request a new token for the own user.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.RenewToken(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var validateTokenCmd = &cobra.Command{
	Use:   "validate",
	Short: "Validate the token of the own user and return its user-context.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ValidateToken(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var tokenCmd = &cobra.Command{
	Use:   "token",
	Short: "Manage the token of the own user.",
}

func Init_Common_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(getVersionCmd)

	rootCmd.AddCommand(getEndpointsCmd)

	rootCmd.AddCommand(tokenCmd)

	tokenCmd.AddCommand(renewTokenCmd)

	tokenCmd.AddCommand(validateTokenCmd)
}
