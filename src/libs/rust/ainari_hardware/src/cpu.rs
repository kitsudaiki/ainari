// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub fn is_amd_ryzen() -> io::Result<bool> {
    let mut file = File::open("/proc/cpuinfo")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let is_amd_ryzen = contents
        .lines()
        .any(|line| line.to_lowercase().contains("amd") && line.to_lowercase().contains("ryzen"));

    Ok(is_amd_ryzen)
}

pub fn is_hyperthreading_enabled() -> io::Result<bool> {
    let mut file = File::open("/sys/devices/system/cpu/smt/active")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim() == "1")
}

pub fn is_hyperthreading_supported() -> io::Result<bool> {
    let mut file = File::open("/sys/devices/system/cpu/smt/control")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim() != "notsupported")
}

pub fn get_temperature(pkg_file_id: usize) -> io::Result<f64> {
    // ryzen cpus neeed speacial handling
    match is_amd_ryzen() {
        Ok(is_ryzen) => {
            if is_ryzen {
                return get_amd_ryzen_teperature(pkg_file_id);
            }
        }
        Err(_) => {
            return Ok(0.0);
        }
    };

    // get temparature for other cpu-types
    let thermal_path = format!("/sys/class/thermal/thermal_zone{pkg_file_id}/temp");
    let file_path = Path::new(thermal_path.as_str());
    match fs::read_to_string(file_path) {
        Ok(content) => {
            if let Ok(temp) = content.trim().parse::<i64>() {
                Ok(temp as f64 / 1000.0)
            } else {
                Ok(0.0)
            }
        }
        Err(e) => Err(e),
    }
}

pub fn get_number_of_cpu_packages() -> io::Result<usize> {
    let path = format!("/sys/devices/system/node/possible");
    let file_path = Path::new(path.as_str());
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(get_range_info(content.as_str().trim())),
        Err(e) => Err(e),
    }
}

pub fn get_number_of_cpu_threads() -> io::Result<usize> {
    let path = format!("/sys/devices/system/cpu/present");
    let file_path = Path::new(path.as_str());
    match fs::read_to_string(file_path) {
        Ok(content) => Ok(get_range_info(content.as_str().trim())),
        Err(e) => Err(e),
    }
}

pub fn get_cpu_sibling_id(thread_id: usize) -> io::Result<usize> {
    // siblings exists only when hyperthreading is enabled
    match is_hyperthreading_enabled() {
        Ok(enabled) => {
            if enabled == false {
                return Ok(thread_id);
            }
        }
        // error here can also mean, that hyperthreading is not available on the system and so the file doesn't exist
        Err(e) => {
            return Err(e);
        }
    }

    // get list of siblings for the thread
    let path = format!("/sys/devices/system/cpu/cpu{thread_id}/topology/thread_siblings_list");
    let file_path = Path::new(path.as_str());
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => {
            return Ok(0);
        }
    };

    // process file-content
    let numbers: Result<Vec<usize>, _> = content
        .as_str()
        .trim()
        .split(',')
        .map(|num_str| num_str.parse::<usize>())
        .collect();

    let values = match numbers {
        Ok(values) => values,
        Err(_) => {
            return Ok(thread_id);
        }
    };

    // 2 values in the file are required. that thread-id itself and its sibling-id
    if values.len() < 2 {
        return Ok(thread_id);
    }

    // get id of the sibling
    if values[0] == thread_id {
        Ok(values[1])
    } else {
        Ok(values[0])
    }
}

pub fn get_current_minimum_speed(thread_id: usize) -> io::Result<usize> {
    get_speed(thread_id, "scaling_min_freq")
}

pub fn get_current_maximum_speed(thread_id: usize) -> io::Result<usize> {
    get_speed(thread_id, "scaling_max_freq")
}

pub fn get_current_speed(thread_id: usize) -> io::Result<usize> {
    get_speed(thread_id, "scaling_cur_freq")
}

pub fn get_minimum_speed(thread_id: usize) -> io::Result<usize> {
    get_speed(thread_id, "cpuinfo_min_freq")
}

pub fn get_maximum_speed(thread_id: usize) -> io::Result<usize> {
    get_speed(thread_id, "cpuinfo_max_freq")
}

pub fn get_package_id(thread_id: usize) -> io::Result<usize> {
    let path = format!("/sys/devices/system/cpu/cpu{thread_id}/topology/physical_package_id");
    let file_path = Path::new(path.as_str());
    match fs::read_to_string(file_path) {
        Ok(content) => {
            if let Ok(val) = content.trim().parse::<usize>() {
                Ok(val)
            } else {
                Ok(0)
            }
        }
        Err(e) => Err(e),
    }
}

pub fn get_core_id(thread_id: usize) -> io::Result<usize> {
    let path = format!("/sys/devices/system/cpu/cpu{thread_id}/topology/core_id");
    let file_path = Path::new(path.as_str());
    match fs::read_to_string(file_path) {
        Ok(content) => {
            if let Ok(val) = content.trim().parse::<usize>() {
                Ok(val)
            } else {
                Ok(0)
            }
        }
        Err(e) => Err(e),
    }
}

fn get_speed(thread_id: usize, speed_type: &str) -> io::Result<usize> {
    let path = format!("/sys/devices/system/cpu/cpu{thread_id}/cpufreq/{speed_type}");
    let file_path = Path::new(path.as_str());
    match fs::read_to_string(file_path) {
        Ok(content) => {
            if let Ok(val) = content.trim().parse::<usize>() {
                Ok(val)
            } else {
                Ok(0)
            }
        }
        Err(_) => Ok(0),
    }
}

pub fn get_pkg_temperature_ids() -> Result<Vec<usize>, String> {
    let mut ids: Vec<usize> = Vec::new();
    let mut counter = 0;
    let base_path = "/sys/class/thermal/thermal_zone";
    let mut found = false;

    loop {
        let file_path = format!("{}{}", base_path, counter);
        let type_path = format!("{}/type", file_path);

        // break-rule to avoid endless-loop
        if !Path::new(&type_path).exists() {
            if !found {
                return Err(
                    "No files found with relevant temperature-information about the CPU"
                        .to_string(),
                );
            }
            return Ok(ids);
        }

        // get type-information behind the id
        match fs::read_to_string(&type_path) {
            // check if the id belongs to the temperature of the cpu-package
            Ok(content) if content.trim() == "x86_pkg_temp" => {
                ids.push(counter);
                found = true;
            }
            Ok(_) => {} // Ignore non-matching entries
            Err(_) => return Err("Failed to read temperature type information".to_string()),
        }

        counter += 1;
    }
}

fn find_k10temp_hwmon() -> io::Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    let hwmon_root = Path::new("/sys/class/hwmon");

    for entry in fs::read_dir(hwmon_root)? {
        let entry = entry?;
        let path = entry.path();
        let name_file = path.join("name");
        if name_file.exists() {
            let name = fs::read_to_string(name_file)?.trim().to_string();
            if name == "k10temp" {
                results.push(path);
            }
        }
    }

    Ok(results)
}

fn get_amd_ryzen_teperature(pkg_file_id: usize) -> io::Result<f64> {
    let k10temp = match find_k10temp_hwmon() {
        Ok(dirs) => dirs,
        Err(_) => Vec::new(),
    };

    if pkg_file_id >= k10temp.len() {
        let error = format!("package-id {pkg_file_id} is too big.");
        return Err(io::Error::new(io::ErrorKind::Other, error));
    }

    let mut file_path: PathBuf = k10temp[pkg_file_id].clone();
    file_path.push("temp1_input");
    match fs::read_to_string(file_path) {
        Ok(content) => {
            if let Ok(temp) = content.trim().parse::<i64>() {
                Ok(temp as f64 / 1000.0)
            } else {
                Ok(0.0)
            }
        }
        Err(_) => Ok(0.0),
    }
}

fn get_range_info(input: &str) -> usize {
    if let Some(second_part) = input.split('-').nth(1) {
        if let Ok(value) = second_part.parse::<usize>() {
            value + 1
        } else {
            1
        }
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu() {
        let is_ryzen = match is_amd_ryzen() {
            Ok(is_ryzen) => is_ryzen,
            Err(_) => false,
        };
        println!("Is cpu an AMD Ryzen: {}", is_ryzen);

        println!(
            "Hyperthreading enabled: {}",
            is_hyperthreading_enabled().unwrap()
        );
        println!(
            "Hyperthreading supported: {}",
            is_hyperthreading_supported().unwrap()
        );

        println!(
            "Number of cpu-packages: {}",
            get_number_of_cpu_packages().unwrap()
        );
        println!(
            "Number of cpu-threads: {}",
            get_number_of_cpu_threads().unwrap()
        );
        println!(
            "Get sibling-id for thread 2: {}",
            get_cpu_sibling_id(2).unwrap()
        );
        println!("Get core-id for thread 2: {}", get_core_id(2).unwrap());
        println!(
            "Get package-id for thread 2: {}",
            get_package_id(2).unwrap()
        );

        println!(
            "Current minimum speed: {}",
            get_current_minimum_speed(0).unwrap()
        );
        println!(
            "Current maximum speed: {}",
            get_current_maximum_speed(0).unwrap()
        );
        println!("Current speed: {}", get_current_speed(0).unwrap());
        println!("Minimum speed: {}", get_minimum_speed(0).unwrap());
        println!("Maximum speed: {}", get_maximum_speed(0).unwrap());

        println!("Temperature: {}", get_temperature(0).unwrap_or(0.0));
    }
}
