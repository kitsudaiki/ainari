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
	columnName           string
	rowOffset            int
	numberOfRows         int
	inputFilePath        string
	labelFilePath        string
	referenceDatasetUuid string
)

var createMnistDatasetCmd = &cobra.Command{
	Use:   "mnist -i INPUT_FILE_PATH -l LABEL_FILE_PATH DATASET_NAME",
	Short: "Upload new mnist dataset.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		datasetName := args[0]
		content, err := ainari_sdk.CreateMnistDataset(context, datasetName, inputFilePath, labelFilePath)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createCsvDatasetCmd = &cobra.Command{
	Use:   "csv -i INPUT_FILE_PATH DATASET_NAME",
	Short: "Upload new csv dataset.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		datasetName := args[0]
		content, err := ainari_sdk.CreateCsvDataset(context, datasetName, inputFilePath)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var checkDatasetCmd = &cobra.Command{
	Use:   "check -r REFERENCE_DATASET_UUID DATASET_UUID",
	Short: "Check a dataset against a reference.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		datasetUuid := args[0]
		content, err := ainari_sdk.CheckDataset(context, datasetUuid, referenceDatasetUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getDatasetCmd = &cobra.Command{
	Use:   "get DATASET_UUID",
	Short: "Get information of a specific dataset.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		datasetUuid := args[0]
		content, err := ainari_sdk.GetDataset(context, datasetUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listDatasetCmd = &cobra.Command{
	Use:   "list",
	Short: "List all dataset.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListDataset(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["datasets"].([]interface{}))
	},
}

var deleteDatasetCmd = &cobra.Command{
	Use:   "delete DATASET_UUID",
	Short: "Delete a specific dataset from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		datasetUuid := args[0]
		_, err = ainari_sdk.DeleteDataset(context, datasetUuid)
		if err == nil {
			fmt.Printf("successfully deleted dataset '%v'\n", datasetUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var downloadDatasetContentCmd = &cobra.Command{
	Use:   "content -c COLUMN_NAME -o ROW_OFFSET -n NUMBER_OF_ROWS DATASET_UUID",
	Short: "Download content of a specific dataset.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		datasetUuid := args[0]
		content, err := ainari_sdk.DownloadDatasetContent(context, datasetUuid, columnName, numberOfRows, rowOffset)
		if err == nil {
			data := content["data"].([]interface{})
			ainarictl_common.PrintValueList(data, rowOffset)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var datasetCmd = &cobra.Command{
	Use:   "dataset",
	Short: "Manage dataset.",
}

var createDatasetCmd = &cobra.Command{
	Use:   "create",
	Short: "Upload datasets.",
}

func Init_Dataset_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(datasetCmd)

	datasetCmd.AddCommand(createDatasetCmd)

	createDatasetCmd.AddCommand(createMnistDatasetCmd)
	createMnistDatasetCmd.Flags().StringVarP(&inputFilePath, "input", "i", "", "Path to file with input-data (mandatory)")
	createMnistDatasetCmd.Flags().StringVarP(&labelFilePath, "label", "l", "", "Path to file with label-data (mandatory)")
	createMnistDatasetCmd.MarkFlagRequired("input")
	createMnistDatasetCmd.MarkFlagRequired("label")

	createDatasetCmd.AddCommand(createCsvDatasetCmd)
	createCsvDatasetCmd.Flags().StringVarP(&inputFilePath, "input", "i", "", "Path to file with input-data (mandatory)")
	createCsvDatasetCmd.MarkFlagRequired("input")

	datasetCmd.AddCommand(checkDatasetCmd)
	checkDatasetCmd.Flags().StringVarP(&referenceDatasetUuid, "reference", "r", "", "UUID of the dataset, which works as reference (mandatory)")
	checkDatasetCmd.MarkFlagRequired("reference")

	datasetCmd.AddCommand(downloadDatasetContentCmd)
	downloadDatasetContentCmd.Flags().StringVarP(&columnName, "column", "c", "", "Name of column to download (mandatory)")
	downloadDatasetContentCmd.Flags().IntVarP(&rowOffset, "offset", "o", 0, "Number of rows to offset (mandatory)")
	downloadDatasetContentCmd.Flags().IntVarP(&numberOfRows, "rows", "n", 1, "Number of rows to download (mandatory)")
	downloadDatasetContentCmd.MarkFlagRequired("column")
	downloadDatasetContentCmd.MarkFlagRequired("rows")

	datasetCmd.AddCommand(getDatasetCmd)

	datasetCmd.AddCommand(listDatasetCmd)

	datasetCmd.AddCommand(deleteDatasetCmd)
}
