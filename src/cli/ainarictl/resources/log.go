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
	ainarictl_common "ainarictl/common"
	"fmt"
	"os"

	ainari_sdk "github.com/kitsudaiki/ainari"
	"github.com/spf13/cobra"
)

// optional filters of the list-commands of the error- and audit-log
var logListFilter ainari_sdk.LogListFilter

var getErrorLogCmd = &cobra.Command{
	Use:   "get ENTRY_UUID",
	Short: "Get a specific entry of the error-log (admin only).",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		entryUuid := args[0]
		content, err := ainari_sdk.GetErrorLog(context, entryUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listErrorLogCmd = &cobra.Command{
	Use:   "list",
	Short: "List the entries of the error-log, the newest first (admin only).",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListErrorLog(context, logListFilter)
		if err == nil {
			ainarictl_common.PrintList(content["entries"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var errorLogCmd = &cobra.Command{
	Use:   "error_log",
	Short: "Read the central error-log of all components, which is collected by omikuji.",
}

var getAuditLogCmd = &cobra.Command{
	Use:   "get ENTRY_UUID",
	Short: "Get a specific entry of the audit-log (admin only).",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		entryUuid := args[0]
		content, err := ainari_sdk.GetAuditLog(context, entryUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listAuditLogCmd = &cobra.Command{
	Use:   "list",
	Short: "List the entries of the audit-log, the newest first (admin only).",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListAuditLog(context, logListFilter)
		if err == nil {
			ainarictl_common.PrintList(content["entries"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var auditLogCmd = &cobra.Command{
	Use:   "audit_log",
	Short: "Read the central audit-log of all components, which is collected by omikuji.",
}

// addLogListFlags adds the optional filters to a list-command of the error- or audit-log.
func addLogListFlags(cmd *cobra.Command) {
	cmd.Flags().StringVarP(&logListFilter.ProjectId, "project", "p", "", "Only list the entries of this project (optional)")
	cmd.Flags().StringVarP(&logListFilter.UserId, "user", "u", "", "Only list the entries of this user (optional)")
	cmd.Flags().StringVar(&logListFilter.HostName, "host", "", "Only list the entries of this host (optional)")
	cmd.Flags().IntVarP(&logListFilter.Limit, "limit", "l", 0, "Maximum number of entries, 100 if not set (optional)")
}

func Init_Log_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(errorLogCmd)

	errorLogCmd.AddCommand(getErrorLogCmd)

	errorLogCmd.AddCommand(listErrorLogCmd)
	addLogListFlags(listErrorLogCmd)

	rootCmd.AddCommand(auditLogCmd)

	auditLogCmd.AddCommand(getAuditLogCmd)

	auditLogCmd.AddCommand(listAuditLogCmd)
	addLogListFlags(listAuditLogCmd)
}
