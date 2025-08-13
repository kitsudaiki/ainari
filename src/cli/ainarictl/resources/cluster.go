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

var (
	templatePath   string
	checkpointName string
	clusterMode    string
)

var createClusterCmd = &cobra.Command{
	Use:   "create -t TEMPLATE_PATH NAME",
	Short: "Create a new cluster.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterName := args[0]
		templateContent, err := os.ReadFile(templatePath)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.CreateCluster(address, token, clusterName, string(templateContent), ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var getClusterCmd = &cobra.Command{
	Use:   "get CLUSTER_UUID",
	Short: "Get information of a specific cluster.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		content, err := ainari_sdk.GetCluster(address, token, clusterUuid, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var listClusterCmd = &cobra.Command{
	Use:   "list",
	Short: "List all cluster.",
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		content, err := ainari_sdk.ListCluster(address, token, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["clusters"].([]interface{}))
	},
}

var deleteClusterCmd = &cobra.Command{
	Use:   "delete CLUSTER_UUID",
	Short: "Delete a specific cluster from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		_, err := ainari_sdk.DeleteCluster(address, token, clusterUuid, ainarictl_common.DisableTlsVerification)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		fmt.Printf("successfully deleted cluster '%v'\n", clusterUuid)
	},
}

var clusterCmd = &cobra.Command{
	Use:   "cluster",
	Short: "Manage cluster.",
}

func Init_Cluster_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(clusterCmd)

	clusterCmd.AddCommand(createClusterCmd)
	createClusterCmd.Flags().StringVarP(&templatePath, "template", "t", "", "Cluster Template (mandatory)")
	createClusterCmd.MarkFlagRequired("template")

	clusterCmd.AddCommand(getClusterCmd)

	clusterCmd.AddCommand(listClusterCmd)

	clusterCmd.AddCommand(deleteClusterCmd)
}
