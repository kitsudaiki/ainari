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
	columnName         string
	rowOffset          int
	numberOfRows       int
	inputFilePath      string
	labelFilePath      string
	referenceImageUuid string
	imageColumn        string
	referenceColumn    string
)

var createMnistImageCmd = &cobra.Command{
	Use:   "mnist -i INPUT_FILE_PATH -l LABEL_FILE_PATH IMAGE_NAME",
	Short: "Upload new mnist image.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageName := args[0]
		content, err := ainari_sdk.CreateMnistImage(context, imageName, inputFilePath, labelFilePath)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createDiskImageCmd = &cobra.Command{
	Use:   "disk -i INPUT_FILE_PATH IMAGE_NAME",
	Short: "Upload new disk-image, which is used as boot-disk of a virtual machine.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageName := args[0]
		content, err := ainari_sdk.CreateDiskImage(context, imageName, inputFilePath)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createCsvImageCmd = &cobra.Command{
	Use:   "csv -i INPUT_FILE_PATH IMAGE_NAME",
	Short: "Upload new csv image.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageName := args[0]
		content, err := ainari_sdk.CreateCsvImage(context, imageName, inputFilePath)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var checkImageCmd = &cobra.Command{
	Use:   "check -c IMAGE_COLUMN -r REFERENCE_IMAGE_UUID -R REFERENCE_COLUMN IMAGE_UUID",
	Short: "Check a column of an image against a column of a reference-image.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageUuid := args[0]
		content, err := ainari_sdk.CheckImage(context, imageUuid, imageColumn, referenceImageUuid, referenceColumn)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getImageCmd = &cobra.Command{
	Use:   "get IMAGE_UUID",
	Short: "Get information of a specific image.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageUuid := args[0]
		content, err := ainari_sdk.GetImage(context, imageUuid)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listImageCmd = &cobra.Command{
	Use:   "list",
	Short: "List all image.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.ListImage(context)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		ainarictl_common.PrintList(content["images"].([]interface{}))
	},
}

var deleteImageCmd = &cobra.Command{
	Use:   "delete IMAGE_UUID",
	Short: "Delete a specific image from the backend.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageUuid := args[0]
		_, err = ainari_sdk.DeleteImage(context, imageUuid)
		if err == nil {
			fmt.Printf("successfully deleted image '%v'\n", imageUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var downloadImageContentCmd = &cobra.Command{
	Use:   "content -c COLUMN_NAME -o ROW_OFFSET -n NUMBER_OF_ROWS IMAGE_UUID",
	Short: "Download content of a specific image.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		imageUuid := args[0]
		content, err := ainari_sdk.DownloadImageContent(context, imageUuid, columnName, numberOfRows, rowOffset)
		if err == nil {
			data := content["data"].([]interface{})
			ainarictl_common.PrintValueList(data, rowOffset)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}


var getImageCountCmd = &cobra.Command{
	Use:   "count",
	Short: "Get the number of images of the project.",
	Run: func(cmd *cobra.Command, args []string) {
		context, err := Login()
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := ainari_sdk.GetImageCount(context)
		if err == nil {
			ainarictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var imageCmd = &cobra.Command{
	Use:   "image",
	Short: "Manage image.",
}

var createImageCmd = &cobra.Command{
	Use:   "create",
	Short: "Upload images.",
}

func Init_Image_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(imageCmd)

	imageCmd.AddCommand(createImageCmd)

	createImageCmd.AddCommand(createMnistImageCmd)
	createMnistImageCmd.Flags().StringVarP(&inputFilePath, "input", "i", "", "Path to file with input-data (mandatory)")
	createMnistImageCmd.Flags().StringVarP(&labelFilePath, "label", "l", "", "Path to file with label-data (mandatory)")
	createMnistImageCmd.MarkFlagRequired("input")
	createMnistImageCmd.MarkFlagRequired("label")

	createImageCmd.AddCommand(createDiskImageCmd)
	createDiskImageCmd.Flags().StringVarP(&inputFilePath, "input", "i", "", "Path to the disk-image-file (mandatory)")
	createDiskImageCmd.MarkFlagRequired("input")

	createImageCmd.AddCommand(createCsvImageCmd)
	createCsvImageCmd.Flags().StringVarP(&inputFilePath, "input", "i", "", "Path to file with input-data (mandatory)")
	createCsvImageCmd.MarkFlagRequired("input")

	imageCmd.AddCommand(checkImageCmd)
	checkImageCmd.Flags().StringVarP(&imageColumn, "column", "c", "", "Name of the column of the image to check (mandatory)")
	checkImageCmd.Flags().StringVarP(&referenceImageUuid, "reference", "r", "", "UUID of the image, which works as reference (mandatory)")
	checkImageCmd.Flags().StringVarP(&referenceColumn, "reference_column", "R", "", "Name of the column of the reference-image (mandatory)")
	checkImageCmd.MarkFlagRequired("column")
	checkImageCmd.MarkFlagRequired("reference")
	checkImageCmd.MarkFlagRequired("reference_column")

	imageCmd.AddCommand(downloadImageContentCmd)
	downloadImageContentCmd.Flags().StringVarP(&columnName, "column", "c", "", "Name of column to download (mandatory)")
	downloadImageContentCmd.Flags().IntVarP(&rowOffset, "offset", "o", 0, "Number of rows to offset (mandatory)")
	downloadImageContentCmd.Flags().IntVarP(&numberOfRows, "rows", "n", 1, "Number of rows to download (mandatory)")
	downloadImageContentCmd.MarkFlagRequired("column")
	downloadImageContentCmd.MarkFlagRequired("rows")

	imageCmd.AddCommand(getImageCmd)

	imageCmd.AddCommand(listImageCmd)

	imageCmd.AddCommand(deleteImageCmd)

	imageCmd.AddCommand(getImageCountCmd)
}
