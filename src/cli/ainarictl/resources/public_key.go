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
	"strings"

	ainari_sdk "github.com/kitsudaiki/ainari"
	"github.com/spf13/cobra"
)

var (
	publicKeyPath string
)

var uploadPublicKeyCmd = &cobra.Command{
	Use:   "upload -k PUBLIC_KEY_FILE PUBLIC_KEY_NAME",
	Short: "Upload new ssh-public-key to omamori.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		// read the key before the login, to fail early on a broken file
		fileContent, err := os.ReadFile(publicKeyPath)
		if err != nil {
			fmt.Printf("failed to read public-key-file '%v': %v\n", publicKeyPath, err)
			os.Exit(1)
		}
		publicKey := strings.TrimSpace(string(fileContent))

		// avoid that a private-key is send to the backend by mistake
		if strings.Contains(publicKey, "PRIVATE KEY") {
			fmt.Printf("file '%v' contains a private-key, but only public-keys are allowed\n", publicKeyPath)
			os.Exit(1)
		}

		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		publicKeyName := args[0]
		content, err := ainari_sdk.UploadPublicKey(context, publicKeyName, publicKey)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getPublicKeyCmd = &cobra.Command{
	Use:   "get PUBLIC_KEY_UUID",
	Short: "Get information of a specific ssh-public-key.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		publicKeyUuid := args[0]
		content, err := ainari_sdk.GetPublicKey(context, publicKeyUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listPublicKeyCmd = &cobra.Command{
	Use:   "list",
	Short: "List all ssh-public-keys.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListPublicKey(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["public_keys"].([]interface{}))
	},
}

var deletePublicKeyCmd = &cobra.Command{
	Use:   "delete PUBLIC_KEY_UUID",
	Short: "Delete a specific ssh-public-key from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		publicKeyUuid := args[0]
		_, err = ainari_sdk.DeletePublicKey(context, publicKeyUuid)
		if err == nil {
			fmt.Printf("successfully deleted public-key '%v'\n", publicKeyUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var publicKeyCmd = &cobra.Command{
	Use:   "public_key",
	Short: "Manage ssh-public-keys.",
}

func Init_PublicKey_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(publicKeyCmd)

	publicKeyCmd.AddCommand(uploadPublicKeyCmd)
	uploadPublicKeyCmd.Flags().StringVarP(&publicKeyPath, "key", "k", "", "Path to the local file with the ssh-public-key to upload (mandatory)")
	uploadPublicKeyCmd.MarkFlagRequired("key")

	publicKeyCmd.AddCommand(getPublicKeyCmd)

	publicKeyCmd.AddCommand(listPublicKeyCmd)

	publicKeyCmd.AddCommand(deletePublicKeyCmd)
}
