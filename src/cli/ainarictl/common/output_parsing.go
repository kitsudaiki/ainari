/**
 * @author      Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright   Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>
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

package ainarictl_common

import (
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"strconv"
	"strings"
	"sort"

	"github.com/olekukonko/tablewriter"
	"github.com/olekukonko/tablewriter/tw"
)

var PrintAsJson bool = false
var DisableTlsVerification bool = false

func PrintSingle(input map[string]interface{}) {
	if PrintAsJson {
		jsonData, _ := json.MarshalIndent(input, "", "    ")
		fmt.Println(string(jsonData))
		return
	}

	// filter header-keys
	header_names := []string{}
	for key := range input {
		header_names = append(header_names, key)
	}
	sort.Strings(header_names)
	
	table := tablewriter.NewWriter(os.Stdout)

	// fill body of table
	for _, element := range header_names {
		v := input[element]
		lineData := []string{}
		lineData = append(lineData, strings.ToUpper(strings.ReplaceAll(element, "_", " ")))
		val := fmt.Sprintf("%v", v)
		if reflect.ValueOf(v).Kind() == reflect.Map {
			jsonData, _ := json.Marshal(v)
			lineData = append(lineData, string(jsonData))
		} else {
			lineData = append(lineData, val)
		}
		table.Append(lineData)
	}

	table.Configure(func(config *tablewriter.Config) {
		config.Row.Formatting.Alignment = tw.AlignLeft
	})
	// table.EnableBorder(false)
	table.Render()
}

func PrintList(input []interface{}) {
	if PrintAsJson {
		jsonData, _ := json.MarshalIndent(input, "", "    ")
		fmt.Println(string(jsonData))
		return
	}

	// handle empty results
	if len(input) == 0 {
		fmt.Println("no values to print")
		return
	}

	// filter header-keys
	header_names := []string{}
	for key := range input[0].(map[string]interface{}) {
		header_names = append(header_names, key)
	}
	sort.Strings(header_names)

	table := tablewriter.NewWriter(os.Stdout)
	table.Header(header_names)

	// fill body of table
	for _, line := range input {
		lineData := []string{}
		for _, header := range header_names {
			val := line.(map[string]interface{})[header]
			if strVal, ok := val.(string); ok {
				lineData = append(lineData, strVal)
			} else {
				str := fmt.Sprintf("%v", val)
				lineData = append(lineData, str)
			}
		}
		table.Append(lineData)
	}

	table.Configure(func(config *tablewriter.Config) {
		config.Row.Formatting.Alignment = tw.AlignLeft
	})
	table.Render()
}

func PrintValueList(data []interface{}, offset int) {
	if PrintAsJson {
		jsonData, _ := json.MarshalIndent(data, "", "    ")
		fmt.Println(string(jsonData))
		return
	}

	table := tablewriter.NewWriter(os.Stdout)

	// fill and add table header
	headerData := []string{}
	headerData = append(headerData, "")
	for i := range len(data[0].([]interface{})) {
		headerData = append(headerData, strconv.Itoa(i))
	}
	table.Header(headerData)

	// fill and add body to table
	for i, line := range data {
		lineData := []string{}
		lineData = append(lineData, fmt.Sprintf("%d", (offset+i)))
		for _, val := range line.([]interface{}) {
			lineData = append(lineData, fmt.Sprintf("%f", val))
		}
		table.Append(lineData)
	}

	table.Configure(func(config *tablewriter.Config) {
		config.Row.Formatting.Alignment = tw.AlignLeft
	})
	table.Render()
}
