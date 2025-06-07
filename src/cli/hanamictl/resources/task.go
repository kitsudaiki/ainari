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

package hanami_resources

import (
	"fmt"
	hanamictl_common "hanamictl/common"
	"os"
	"strings"
	"errors"

	hanami_sdk "github.com/kitsudaiki/OpenHanami"
	"github.com/spf13/cobra"
)

var (
	checkpointUuid string
	inputData      []string
	outputData     []string
	timeLength     int
	numberOfEpochs int
)

func convertTaskIO(input []string) ([]hanami_sdk.TaskInput, error) {
	ret := []hanami_sdk.TaskInput{}

	for _, val := range input {

		// Check if both separator ":" are present
		if !strings.Contains(val, ":") {
			return ret, errors.New("Error: Required separators ':' are missing")
		}

		// Split at ":"
		parts := strings.Split(val, ":")
		if len(parts) != 3 {
			return ret, errors.New("Error: Invalid format before or after ':'")
		}
		datasetUUID := parts[0]
		columnName := parts[1]
		hexagonName := parts[2]

		item := hanami_sdk.TaskInput{
			HexagonName:       hexagonName, 
			DatasetColumnName: columnName, 
			DatasetUuid:       datasetUUID,
		}

		ret = append(ret, item)
	}

	return ret, nil
}

func convertTaskResult(input []string) ([]hanami_sdk.TaskResult, error) {
	ret := []hanami_sdk.TaskResult{}

	for _, val := range input {

		// Check if both separator ":" are present
		if !strings.Contains(val, ":") {
			return ret, errors.New("Error: Required separators ':' are missing")
		}

		// Split at ":"
		parts := strings.Split(val, ":")
		if len(parts) != 2 {
			return ret, errors.New("Error: Invalid format before or after ':'")
		}
		hexagonName := parts[0]
		columnName := parts[1]

		item := hanami_sdk.TaskResult{
			HexagonName:       hexagonName, 
			DatasetColumnName: columnName,
		}

		ret = append(ret, item)
	}

	return ret, nil
}


var createTrainTaskCmd = &cobra.Command{
	Use:   "train -i DATASET_UUID:COLUMN_NAME:HEXAGON_NAME -o DATASET_UUID:COLUMN_NAME:HEXAGON_NAME -e NUMBER_OF_EPOCHS CLUSTER_UUID TASK_NAME",
	Short: "Create a new train task.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskName := args[1]
		taskInput, err := convertTaskIO(inputData)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		taskOutput, err := convertTaskIO(outputData)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := hanami_sdk.CreateTrainTask(address, token, taskName, clusterUuid, taskInput, taskOutput, numberOfEpochs, timeLength, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createRequestTaskCmd = &cobra.Command{
	Use:   "request -i DATASET_UUID:COLUMN_NAME:HEXAGON_NAME -r HEXAGON_NAME:COLUMN_NAME CLUSTER_UUID TASK_NAME",
	Short: "Create a new request task.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskName := args[1]
		taskInput, err := convertTaskIO(inputData)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		taskOutput, err := convertTaskResult(outputData)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}
		content, err := hanami_sdk.CreateRequestTask(address, token, taskName, clusterUuid, taskInput, taskOutput, timeLength, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createCheckpointSaveTaskCmd = &cobra.Command{
	Use:   "checkpoint_create CLUSTER_UUID TASK_NAME",
	Short: "Create a new task to create a checkpoint from a cluster.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskName := args[1]
		content, err := hanami_sdk.CreateCheckpointSaveTask(address, token, taskName, clusterUuid, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var createCheckpointRestoreTaskCmd = &cobra.Command{
	Use:   "checkpoint_restore -c CHECKPOINT_UUID CLUSTER_UUID TASK_NAME",
	Short: "Create a new task to restore a checkpoint into a cluster.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskName := args[1]
		content, err := hanami_sdk.CreateCheckpointRestoreTask(address, token, taskName, clusterUuid, checkpointUuid, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var getTaskCmd = &cobra.Command{
	Use:   "get CLUSTER_UUID TASK_UUID",
	Short: "Get information of a specific task.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskUuid := args[1]
		content, err := hanami_sdk.GetTask(address, token, taskUuid, clusterUuid, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var listTaskCmd = &cobra.Command{
	Use:   "list CLUSTER_UUID",
	Short: "List all task.",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		content, err := hanami_sdk.ListTask(address, token, clusterUuid, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintList(content["tasks"].([]interface{}))
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var deleteTaskCmd = &cobra.Command{
	Use:   "delete CLUSTER_UUID TASK_UUID",
	Short: "Delete a specific task from the backend.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskUuid := args[1]
		_, err := hanami_sdk.DeleteTask(address, token, taskUuid, clusterUuid, hanamictl_common.DisableTlsVerification)
		if err == nil {
			fmt.Printf("successfully deleted task '%v'\n", taskUuid)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var abortTaskCmd = &cobra.Command{
	Use:   "abort CLUSTER_UUID TASK_UUID",
	Short: "Abort a specific task.",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		token := Login()
		address := os.Getenv("HANAMI_ADDRESS")
		clusterUuid := args[0]
		taskUuid := args[1]
		content, err := hanami_sdk.AbortTask(address, token, taskUuid, clusterUuid, hanamictl_common.DisableTlsVerification)
		if err == nil {
			hanamictl_common.PrintSingle(content)
		} else {
			fmt.Println(err)
			os.Exit(1)
		}
	},
}

var taskCmd = &cobra.Command{
	Use:   "task",
	Short: "Manage task.",
}

var createTaskCmd = &cobra.Command{
	Use:   "create",
	Short: "Create new task.",
}

func Init_Task_Commands(rootCmd *cobra.Command) {
	rootCmd.AddCommand(taskCmd)

	taskCmd.AddCommand(createTaskCmd)

	createTaskCmd.AddCommand(createTrainTaskCmd)
	createTrainTaskCmd.Flags().StringSliceVarP(&inputData, "input", "i", []string{}, "Cluster input, which are paris of '-i <DATASET_UUID>:<COLUMN_NAME>:<HEXAGON_NAME>' (mandatory)")
	createTrainTaskCmd.Flags().StringSliceVarP(&outputData, "output", "o", []string{}, "Cluster outputs, which are paris of '-o <DATASET_UUID>:<COLUMN_NAME>:<HEXAGON_NAME>' (mandatory)")
	createTrainTaskCmd.Flags().IntVarP(&timeLength, "time", "t", 1, "Length of a time-series for the input")
	createTrainTaskCmd.Flags().IntVarP(&numberOfEpochs, "epochs", "e", 1, "Number of epochs for the training")
	createTrainTaskCmd.MarkFlagRequired("input")
	createTrainTaskCmd.MarkFlagRequired("output")

	createTaskCmd.AddCommand(createRequestTaskCmd)
	createRequestTaskCmd.Flags().StringSliceVarP(&inputData, "input", "i", []string{}, "Cluster input, which are paris of '-i <DATASET_UUID>:<COLUMN_NAME>:<HEXAGON_NAME>' (mandatory)")
	createRequestTaskCmd.Flags().StringSliceVarP(&outputData, "result", "r", []string{}, "Cluster result, which are paris of '-r <HEXAGON_NAME>:<COLUMN_NAME>' (mandatory)")
	createRequestTaskCmd.Flags().IntVarP(&timeLength, "time", "t", 1, "Length of a time-series for the input")
	createRequestTaskCmd.MarkFlagRequired("input")
	createRequestTaskCmd.MarkFlagRequired("result")

	createTaskCmd.AddCommand(createCheckpointSaveTaskCmd)

	createTaskCmd.AddCommand(createCheckpointRestoreTaskCmd)
	createCheckpointRestoreTaskCmd.Flags().StringVarP(&checkpointUuid, "checkpoint_uuid", "c", "", "Checkpoint UUID UUID (mandatory)")
	createCheckpointRestoreTaskCmd.MarkFlagRequired("checkpoint_uuid")

	taskCmd.AddCommand(getTaskCmd)

	taskCmd.AddCommand(listTaskCmd)

	taskCmd.AddCommand(deleteTaskCmd)

	taskCmd.AddCommand(abortTaskCmd)
}
