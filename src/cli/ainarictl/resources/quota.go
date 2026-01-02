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
    maxCluster    int
    maxDataset    int
    maxCheckpoint int
    maxSecret     int
    maxTaskqueue  int
)

var setQuotaCmd = &cobra.Command{
	Use:   "set USER_ID",
	Short: "Set new quota for a user.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		userId := args[0]

		content, err := ainari_sdk.SetQuota(context, userId, maxCluster, maxDataset, maxCheckpoint, maxSecret, maxTaskqueue)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var getQuotaCmd = &cobra.Command{
	Use:   "get USER_ID",
	Short: "Get information of a specific quota.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		userId := args[0]
		content, err := ainari_sdk.GetQuota(context, userId)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var listQuotaCmd = &cobra.Command{
	Use:   "list",
	Short: "List all quota.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListQuota(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["quotas"].([]interface{}))
	},
}

var quotaCmd = &cobra.Command{
	Use:   "quota",
	Short: "Manage quota.",
}

func Init_Quota_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(quotaCmd)

	quotaCmd.AddCommand(setQuotaCmd)
	setQuotaCmd.Flags().IntVar(&maxCluster, "max_cluster", 0, "Set quota as admin")
	setQuotaCmd.Flags().IntVar(&maxDataset, "max_dataset", 0, "Set quota as admin")
	setQuotaCmd.Flags().IntVar(&maxCheckpoint, "max_checkpoint", 0, "Set quota as admin")
	setQuotaCmd.Flags().IntVar(&maxSecret, "max_secret", 0, "Set quota as admin")
	setQuotaCmd.Flags().IntVar(&maxTaskqueue, "max_taskqueue", 0, "Set quota as admin")

	quotaCmd.AddCommand(getQuotaCmd)

	quotaCmd.AddCommand(listQuotaCmd)
}
