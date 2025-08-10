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

package ainari_sdk

func getPowerData(address, token string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "/v1alpha/power_consumption"
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func getTemperatureData_request(address, token string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "/v1alpha/temperature_production"
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}

func getSpeedData_request(address, token string, skipTlsVerification bool) (map[string]interface{}, error) {
	path := "/v1alpha/speed"
	vars := map[string]interface{}{}
	return SendGet(address, token, path, vars, skipTlsVerification)
}
