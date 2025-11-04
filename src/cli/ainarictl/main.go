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

package main

import (
	"fmt"
	ainarictl_common "ainarictl/common"
	ainari_resources "ainarictl/resources"
	"os"

	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{Use: "ainarictl"}

func init() {
	rootCmd.PersistentFlags().BoolVarP(&ainarictl_common.PrintAsJson, "json_output", "j", false, "Return output as json")
	rootCmd.PersistentFlags().BoolVar(&ainarictl_common.DisableTlsVerification, "insecure", false, "Disable the TLS-verification")

	ainari_resources.Init_Common_Commands(rootCmd)
	ainari_resources.Init_User_Commands(rootCmd)
	ainari_resources.Init_Project_Commands(rootCmd)
	ainari_resources.Init_Checkpoint_Commands(rootCmd)
	ainari_resources.Init_Task_Commands(rootCmd)
	ainari_resources.Init_Cluster_Commands(rootCmd)
	ainari_resources.Init_Dataset_Commands(rootCmd)
	ainari_resources.Init_Proxy_Commands(rootCmd)
	ainari_resources.Init_Host_Commands(rootCmd)
	ainari_resources.Init_Secret_Commands(rootCmd)
	ainari_resources.Init_Quota_Commands(rootCmd)
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}
