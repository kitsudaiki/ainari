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
	secretPayload string
)

var createSecretCmd = &cobra.Command{
	Use:   "create -s SECRET SECRET_NAME",
	Short: "Upload new secret to omamori.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		secretName := args[0]
		content, err := ainari_sdk.CreateSecret(context, secretName, secretPayload)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getSecretCmd = &cobra.Command{
	Use:   "get SECRET_UUID",
	Short: "Get information of a specific secret.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		secretUuid := args[0]
		content, err := ainari_sdk.GetSecret(context, secretUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getSecretWithPayloadCmd = &cobra.Command{
	Use:   "get-payload SECRET_UUID",
	Short: "Get information of a specific secret with the secret-payload.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		secretUuid := args[0]
		content, err := ainari_sdk.GetSecretWitchPayload(context, secretUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listSecretCmd = &cobra.Command{
	Use:   "list",
	Short: "List all secret.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListSecret(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["secrets"].([]interface{}))
	},
}

var deleteSecretCmd = &cobra.Command{
	Use:   "delete SECRET_UUID",
	Short: "Delete a specific secret from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		secretUuid := args[0]
		_, err = ainari_sdk.DeleteSecret(context, secretUuid)
		if err == nil {
			fmt.Printf("successfully deleted secret '%v'\n", secretUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var secretCmd = &cobra.Command{
	Use:   "secret",
	Short: "Manage secret.",
}

func Init_Secret_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(secretCmd)

	secretCmd.AddCommand(createSecretCmd)
	createSecretCmd.Flags().StringVarP(&secretPayload, "payload", "p", "", "Secret-payload to upload (mandatory)")
	createSecretCmd.MarkFlagRequired("payload")

	secretCmd.AddCommand(getSecretCmd)

	secretCmd.AddCommand(getSecretWithPayloadCmd)

	secretCmd.AddCommand(listSecretCmd)

	secretCmd.AddCommand(deleteSecretCmd)
}
