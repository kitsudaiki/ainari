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

var getSnapshotCmd = &cobra.Command{
	Use:   "get SNAPSHOT_UUID",
	Short: "Get information of a specific snapshot.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		snapshotUuid := args[0]
		content, err := ainari_sdk.GetSnapshot(context, snapshotUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var listSnapshotCmd = &cobra.Command{
	Use:   "list",
	Short: "List all snapshot.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListSnapshot(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["snapshots"].([]interface{}))
	},
}

var deleteSnapshotCmd = &cobra.Command{
	Use:   "delete SNAPSHOT_UUID",
	Short: "Delete a specific snapshot from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		snapshotUuid := args[0]
		_, err = ainari_sdk.DeleteSnapshot(context, snapshotUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		fmt.Printf("successfully deleted snapshot '%v'\n", snapshotUuid)
	},
}


var getSnapshotCountCmd = &cobra.Command{
	Use:   "count",
	Short: "Get the number of snapshots of the project.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.GetSnapshotCount(context)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var snapshotCmd = &cobra.Command{
	Use:   "snapshot",
	Short: "Manage snapshot.",
}

func Init_Snapshot_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(snapshotCmd)

	snapshotCmd.AddCommand(getSnapshotCmd)

	snapshotCmd.AddCommand(listSnapshotCmd)

	snapshotCmd.AddCommand(deleteSnapshotCmd)

	snapshotCmd.AddCommand(getSnapshotCountCmd)
}
