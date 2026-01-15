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
	templatePath   string
	checkpointName string
	modelMode    string
)

var createModelCmd = &cobra.Command{
	Use:   "create -t TEMPLATE_PATH NAME",
	Short: "Create a new model.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		modelName := args[0]
		templateContent, err := os.ReadFile(templatePath)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.CreateModel(context, modelName, string(templateContent))
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var getModelCmd = &cobra.Command{
	Use:   "get CLUSTER_UUID",
	Short: "Get information of a specific model.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		modelUuid := args[0]
		content, err := ainari_sdk.GetModel(context, modelUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintSingle(content)
	},
}

var listModelCmd = &cobra.Command{
	Use:   "list",
	Short: "List all model.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListModel(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["models"].([]interface{}))
	},
}

var deleteModelCmd = &cobra.Command{
	Use:   "delete CLUSTER_UUID",
	Short: "Delete a specific model from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		modelUuid := args[0]
		_, err = ainari_sdk.DeleteModel(context, modelUuid)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		fmt.Printf("successfully deleted model '%v'\n", modelUuid)
	},
}

var modelCmd = &cobra.Command{
	Use:   "model",
	Short: "Manage model.",
}

func Init_Model_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(modelCmd)

	modelCmd.AddCommand(createModelCmd)
	createModelCmd.Flags().StringVarP(&templatePath, "template", "t", "", "Model Template (mandatory)")
	createModelCmd.MarkFlagRequired("template")

	modelCmd.AddCommand(getModelCmd)

	modelCmd.AddCommand(listModelCmd)

	modelCmd.AddCommand(deleteModelCmd)
}
