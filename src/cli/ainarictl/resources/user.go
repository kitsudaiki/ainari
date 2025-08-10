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
	"strings"
	"syscall"

	ainari_sdk "github.com/kitsudaiki/Ainari"

	"github.com/spf13/cobra"
	"golang.org/x/term"
)

var (
	userName   string
	passphrase string
	isAdmin    bool
)

var createUserCmd = &cobra.Command{
	Use:   "create USER_ID",
	Short: "Create a new user.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		var err error
		if len(passphrase) == 0 {
			fmt.Print("Enter Passphrase: ")
			bytePassphrase1, err := term.ReadPassword(syscall.Stdin)
			if err != nil {
				fmt.Println("Failed to read passphrase input")
				os.Exit(1)
			}
			passphrase1 := strings.TrimSpace(string(bytePassphrase1))

			fmt.Print("\n")
			fmt.Print("Enter Passphrase again: ")
			bytePassphrase2, err := term.ReadPassword(syscall.Stdin)
			if err != nil {
				fmt.Println("Failed to read passphrase input")
				os.Exit(1)
			}
			passphrase2 := strings.TrimSpace(string(bytePassphrase2))

			fmt.Print("\n")
			if passphrase1 != passphrase2 {
				fmt.Println("Mismatch between the two entered passphrases")
				os.Exit(1)
			}

			passphrase = passphrase1
		}
		userId := args[0]

		content, err := ainari_sdk.CreateUser(address, token, userId, userName, passphrase, isAdmin, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var getUserCmd = &cobra.Command{
	Use:   "get USER_ID",
	Short: "Get information of a specific user.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		userId := args[0]
		content, err := ainari_sdk.GetUser(address, token, userId, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var listUserCmd = &cobra.Command{
	Use:   "list",
	Short: "List all user.",
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		content, err := ainari_sdk.ListUser(address, token, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["users"].([]interface{}))
	},
}

var deleteUserCmd = &cobra.Command{
	Use:   "delete USER_ID",
	Short: "Delete a specific user from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		userId := args[0]
		_, err := ainari_sdk.DeleteUser(address, token, userId, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		fmt.Printf("successfully deleted user '%v'\n", userId)
	},
}

var userCmd = &cobra.Command{
	Use:   "user",
	Short: "Manage user.",
}

func Init_User_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(userCmd)

	passphraseFlagText := "Passphrase for the new user. " +
		"If not given by this flag, the passphrase will be automatically requested after entering the command. " +
		"This flag is quite unsave, because this way the passphrase is visible in the command-line and " +
		"printed into the history. So this flag should be only used for automated testing, " +
		"but NEVER in a productive environment."
	userCmd.AddCommand(createUserCmd)
	createUserCmd.Flags().StringVarP(&userName, "name", "n", "", "User name (mandatory)")
	createUserCmd.Flags().StringVarP(&passphrase, "passphrase", "p", "", passphraseFlagText)
	createUserCmd.Flags().BoolVar(&isAdmin, "is_admin", false, "Set user as admin (default: false)")
	createUserCmd.MarkFlagRequired("name")

	userCmd.AddCommand(getUserCmd)

	userCmd.AddCommand(listUserCmd)

	userCmd.AddCommand(deleteUserCmd)
}
