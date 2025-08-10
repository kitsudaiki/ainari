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

use std::collections::HashMap;

use pest::Parser;
use pest_derive::Parser;
use uuid::Uuid;
use rand::Rng;

use super::cluster_meta_structs::*;

use hanami_common::enums::*;
use hanami_common::objects::*;
use hanami_common::error::HanamiError;
use hanami_common::constants::*;
use hanami_common::functions::*;

#[derive(Parser)]
#[grammar = "cluster_parser.pest"]
pub struct ClusterParser;

pub fn parse_cluster_template(name: &String, source_text: &str) -> Result<ClusterMeta, HanamiError> {
    let file_pair = ClusterParser::parse(Rule::file, source_text)
        .map_err(|e| HanamiError::InputError(format!("Failed to parse parsed_cluster-template with error: {}", e)))?
        .next().unwrap();

    let mut version: i32 = 0;
    let mut settings = Settings {
        neuron_cooldown: 10000000000.0,
        refractory_time: 1,
        max_connection_distance: 1,
    };
    let mut hexagons = HashMap::new();
    let mut axons = Vec::new();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for section in file_pair.into_inner() {
        match section.as_rule() {
            Rule::version_r => {
                let number = section.into_inner().next().unwrap();
                version = number.as_str().parse().unwrap();

                if version != 1 {
                    let msg = format!("Version {version} not supported");
                    return Err(HanamiError::InputError(msg));
                }
            }
            Rule::settings_r => {
                for setting_pair in section.into_inner() {
                    match setting_pair.as_rule() {
                        Rule::neuron_cooldown_r => {
                            let float_pair = setting_pair.into_inner().next().unwrap();
                            settings.neuron_cooldown = float_pair.as_str().parse().unwrap();
                        }
                        Rule::refractory_time_r => {
                            let int_pair = setting_pair.into_inner().next().unwrap();
                            settings.refractory_time = int_pair.as_str().parse().unwrap();
                        }
                        Rule::max_connection_distance_r => {
                            let int_pair = setting_pair.into_inner().next().unwrap();
                            settings.max_connection_distance = int_pair.as_str().parse().unwrap();
                        }
                        _ => {}
                    }
                }
            }
            Rule::hexagons_r => {
                for position in section.into_inner().flat_map(|list| list.into_inner()) {
                    if position.as_rule() == Rule::position {
                        let hexagon_meta = HexagonMeta::new(parse_position(position));
                        hexagons.insert(hexagon_meta.uuid.clone(), hexagon_meta);
                    }
                }
            }
            Rule::axons_r => {
                for axon_rule in section.into_inner() {
                    let mut inner = axon_rule.into_inner();
                    let from = parse_position(inner.next().unwrap());
                    let to = parse_position(inner.next().unwrap());
                    axons.push(AxonMeta { from, to });
                }
            }
            Rule::inputs_r => {
                for input_rule in section.into_inner() {
                    let mut inner = input_rule.into_inner();
                    let name = inner.next().unwrap().as_str().to_string();
                    let position = parse_position(inner.next().unwrap());
                    inputs.push(InputMeta::new(name, position));
                }
            }
            Rule::outputs_r => {
                for out in section.into_inner() {
                    let mut inner = out.into_inner();
            
                    let name_str = inner.next().unwrap().as_str().to_string();
                    let position = parse_position(inner.next().unwrap());
            
                    // get optional output-type, "plain" is default
                    let next = inner.next();
                    let extra_field = match next {
                        Some(extra_pair) if extra_pair.as_rule() == Rule::extra_info => {
                            let inner_key = extra_pair.into_inner().next().unwrap();
                            inner_key.as_str()
                        }
                        _ => "plain",
                    };
                
                    // convert output-type
                    let output_type = match extra_field {
                        "plain" => OutputType::PlainOutput,
                        "float" => OutputType::FloatOutput,
                        "int"   => OutputType::IntOutput,
                        "bool"  => OutputType::BoolOutput,
                        _       => {

                            let msg = format!("Invalid output extra value: '{extra_field}'");
                            return Err(HanamiError::InputError(msg));
                        }
                    };

                    outputs.push(OutputMeta::new(name_str, position, output_type));
                }
            }
            _ => {}
        }
    }

    let mut parsed_cluster = ClusterMeta {
        uuid: Uuid::nil(),
        name: name.clone(),
        version,
        settings,
        hexagons,
        axons,
        inputs,
        outputs,
    };

    init_cluster(&mut parsed_cluster)?;

    Ok(parsed_cluster)
}


fn parse_position(pair: pest::iterators::Pair<Rule>) -> Position {
    let mut coords = pair.into_inner().map(|n| n.as_str().parse::<u32>().unwrap());
    Position {
        x: coords.next().unwrap(),
        y: coords.next().unwrap(),
        z: coords.next().unwrap(),
    }
}

fn init_cluster(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> {
    initialize_hexagons(parsed_cluster)?;
    update_axons(parsed_cluster)?;
    initialize_inputs(parsed_cluster)?;
    initialize_outputs(parsed_cluster)?;

    Ok(())
}

fn search_hexagon(hexagons_meta: &HashMap<Uuid, HexagonMeta>, position: &Position, type_name: &str) -> Result<Uuid, HanamiError> {
    for h in hexagons_meta.values() {
        if h.positon == *position {
            return Ok(h.uuid.clone());
        }
    }
    
    let msg = format!("Invalid {type_name} position: {}", position.to_string());
    Err(HanamiError::InputError(msg))
}

fn initialize_inputs(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> {
    for input_meta in parsed_cluster.inputs.iter_mut() {
        let hexagon_uuid  = search_hexagon(&parsed_cluster.hexagons, &input_meta.position, "input")?;
        if let Some(obj) = parsed_cluster.hexagons.get_mut(&hexagon_uuid) {
            obj.is_input = true;
            input_meta.hexagon_uuid = hexagon_uuid.clone();
        } else {
            return Err(HanamiError::Error(format!("Can not find input-hexagon with ID {hexagon_uuid}, even it should exist.")));
        }
    }
    Ok(())
}

fn initialize_outputs(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> {
    for output_meta in parsed_cluster.outputs.iter_mut() {
        let hexagon_uuid  = search_hexagon(&parsed_cluster.hexagons, &output_meta.position, "output")?;
        if let Some(obj) = parsed_cluster.hexagons.get_mut(&hexagon_uuid) {
            obj.is_output = true;
            obj.name = output_meta.name.clone();
            output_meta.hexagon_uuid = hexagon_uuid.clone();
        } else {
            return Err(HanamiError::Error(format!("Can not find output-hexagon with ID {hexagon_uuid}, even it should exist.")));
        }
    }
    Ok(())
}

fn initialize_hexagons(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> {
    update_axons(parsed_cluster)?;
    connect_all_hexagons(parsed_cluster)?;
    initialize_target_hexagon_list(parsed_cluster)?;

    Ok(())
}

fn update_axons(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> {
    for axon in parsed_cluster.axons.clone() {
        let from_id = search_hexagon(&parsed_cluster.hexagons, &axon.from, &"axon with source")?;
        let target_id = search_hexagon(&parsed_cluster.hexagons, &axon.to, &"axon with target")?;

        if let Some(obj) = parsed_cluster.hexagons.get_mut(&from_id) {
            obj.axon_target = target_id;
        }
    }

    Ok(())
}

fn connect_hexagon(
    parsed_cluster: &mut ClusterMeta, 
    hexagon_copy: &HashMap<Uuid, HexagonMeta>,
    source_id: &Uuid, 
    source_pos: &Position, 
    side: usize) -> Result<(), HanamiError> 
{
    let next = get_neighbor_pos(&source_pos, side);
    if next.is_valid() {
        for (target_id, hexagon) in hexagon_copy.iter() {
            let target_pos = &hexagon.positon;
            if *target_pos == next {
                if let Some(source_obj) = parsed_cluster.hexagons.get_mut(&source_id.clone()) {
                    source_obj.neighbors[side] = target_id.clone();
                }
                if let Some(target_obj) = parsed_cluster.hexagons.get_mut(&target_id) {
                    target_obj.neighbors[11 - side] = source_id.clone();
                }
            }
        }
    }

    Ok(())
}

fn connect_all_hexagons(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> 
{
    let hexagon_copy = parsed_cluster.hexagons.clone();
    for (source_id, source_hexagon) in hexagon_copy.iter() {
        let source_pos = &source_hexagon.positon;
        for side in 0..12 {
            connect_hexagon(parsed_cluster, &hexagon_copy, &source_id, source_pos, side)?;
        }
    }

    Ok(())
}

fn go_to_next_hexagon(
    hexagons_static_copy: &HashMap<Uuid, HexagonMeta>, 
    current_hexagon: &HexagonMeta,
    source_id: &Uuid, 
    max_path_length: &mut i32) -> Result<Uuid, HanamiError> 
{
    // check path-length to not go too far
    *max_path_length -= 1;
    if *max_path_length < 0 {
        return Ok(current_hexagon.uuid.clone());
    }

    let chance_for_next = 0.5f32;  // TODO: make hard-coded value configurable
    let num = rand::rng().random_range(0..1000) as f32;
    if (chance_for_next * 1000.0f32 < num) && source_id != &current_hexagon.uuid {
        return Ok(current_hexagon.uuid.clone());
    }

    let mut available_next: Vec<Uuid> = vec![];
    let possible_next_sides = [9, 3, 1, 4, 11, 5, 2];
    for side in possible_next_sides {
        let next_hexagon_id = current_hexagon.neighbors[side].clone();
        if next_hexagon_id != Uuid::nil() {
            available_next.push(next_hexagon_id);
        }
    }

    if available_next.len() == 0 {
        return Ok(current_hexagon.uuid.clone());
    }

    let next_select = rand::rng().random_range(0..available_next.len());
    let selected_next_id = &available_next[next_select];

    if let Some(obj) = hexagons_static_copy.get(selected_next_id) {
        return go_to_next_hexagon(hexagons_static_copy, &obj, &source_id, max_path_length);
    } else {
        return Ok(current_hexagon.uuid.clone());
    }
}

fn initialize_target_hexagon_list(parsed_cluster: &mut ClusterMeta) -> Result<(), HanamiError> {
    let hexagons_static_copy = parsed_cluster.hexagons.clone();
    for (source_uuid, source_hexagon) in parsed_cluster.hexagons.iter_mut() {
        for counter in 0..NUMBER_OF_POSSIBLE_NEXT {
            let mut max_path_length = parsed_cluster.settings.max_connection_distance.clone() as i32;

            // hexagons with a different axon-target have an advantage, so they reduced by one step to be equal to other hexagons
            if source_uuid != &source_hexagon.axon_target  {
                max_path_length -= 1;
            }

            let base_hexagon = &hexagons_static_copy.get(&source_hexagon.axon_target).unwrap();
            let target_hexagon_id = go_to_next_hexagon(&hexagons_static_copy, &base_hexagon, &source_uuid, &mut max_path_length)?;

            // handle result
            if &target_hexagon_id != source_uuid {
                source_hexagon.possible_hexagon_target_ids[counter] = target_hexagon_id;
            } else {
                source_hexagon.possible_hexagon_target_ids[counter] = Uuid::nil();
            }
        }
    }

    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_version() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,2,3 -> 4,5,6; 
        inputs: 
            key1: 1,2,3; 
        outputs: 
            key2: 4,5,6;";
            

        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.version, 1);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }        
    }

    #[test]
    fn test_parse_settings() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 44;
            max_connection_distance: 43;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,2,3 -> 4,5,6; 
        inputs: 
            key1: 1,2,3; 
        outputs: 
            key2: 4,5,6;";
            

        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.settings.neuron_cooldown, 1000000000.0);
                assert_eq!(parsed.settings.refractory_time, 44);
                assert_eq!(parsed.settings.max_connection_distance, 43);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }        
    }
    
    #[test]
    fn test_parse_tests() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,2,3 -> 4,5,6; 
        inputs: 
            key1: 1,2,3; 
        outputs: 
            key2: 4,5,6;";
            

        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.hexagons.len(), 2);

                if let Some(value) = parsed.hexagons.values().nth(0) {
                    let valid = value.positon == Position{x: 1, y: 2, z: 3} || value.positon == Position{x: 4, y: 5, z: 6};
                    assert!(valid);
                } else {
                    assert_eq!(false, true);
                }
                if let Some(value) = parsed.hexagons.values().nth(1) {
                    let valid = value.positon == Position{x: 1, y: 2, z: 3} || value.positon == Position{x: 4, y: 5, z: 6};
                    assert!(valid);
                } else {
                    assert_eq!(false, true);
                }
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }  
    }

    #[test]
    fn test_parse_axons() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            2,2,2; 
            4,5,6; 
        axons: 
            1,2,3 -> 2,2,2; 
            2,2,2 -> 4,5,6; 
        inputs: 
            key1: 1,2,3; 
        outputs: 
            key2: 4,5,6;";
            
        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.axons.len(), 2);
                assert_eq!(parsed.axons[0].from, Position{x: 1, y: 2, z: 3});
                assert_eq!(parsed.axons[0].to,   Position{x: 2, y: 2, z: 2});
                assert_eq!(parsed.axons[1].from, Position{x: 2, y: 2, z: 2});
                assert_eq!(parsed.axons[1].to,   Position{x: 4, y: 5, z: 6});
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_inputs() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
            1,1,1; 
            2,2,2;
            3,3,3;
            4,4,4; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3;
            key4: 4,4,4;";
            
        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.inputs.len(), 2);
                assert_eq!(parsed.inputs[0].name, "key1");
                assert_eq!(parsed.inputs[0].position, Position{x: 1, y: 1, z: 1});
                assert_eq!(parsed.inputs[1].name, "key2");
                assert_eq!(parsed.inputs[1].position, Position{x: 2, y: 2, z: 2});
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_outputs() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
            1,1,1; 
            2,2,2;
            3,3,3;
            4,4,4; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3;
            key4: 4,4,4;";

        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].position, Position{x: 3, y: 3, z: 3});
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].position, Position{x: 4, y: 4, z: 4});
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_outputs_with_extra() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
            1,1,1; 
            2,2,2;
            3,3,3;
            4,4,4; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3 (float);
            key4: 4,4,4;";

        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].position, Position{x: 3, y: 3, z: 3});
                assert_eq!(parsed.outputs[0].output_type, OutputType::FloatOutput);
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].position, Position{x: 4, y: 4, z: 4});
                assert_eq!(parsed.outputs[1].output_type, OutputType::PlainOutput);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_outputs_without_axons() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
            1,1,1; 
            2,2,2;
            3,3,3;
            4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3 (float);
            key4: 4,4,4;";

        match parse_cluster_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].position, Position{x: 3, y: 3, z: 3});
                assert_eq!(parsed.outputs[0].output_type, OutputType::FloatOutput);
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].position, Position{x: 4, y: 4, z: 4});
                assert_eq!(parsed.outputs[1].output_type, OutputType::PlainOutput);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {}", e);
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_empty_input() {
        let name = "test-cluster".to_string();
        let input = "";
        let result = parse_cluster_template(&name, input);
        assert!(result.is_err(), "Empty input should result in an error.");
    }
    
    #[test]
    fn test_invalid_input() {
        let name = "test-cluster".to_string();
        let input = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 ->;
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;";
        let result = parse_cluster_template(&name, input);
        assert!(result.is_err(), "Invalid input should result in an error.");
    }

    #[test]
    fn test_invalid_version() {
        let name = "test-cluster".to_string();
        let input = "version: 2
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3 (float);
            key4: 4,4,4;";
        let result = parse_cluster_template(&name, input);
        assert!(result.is_err(), "Invalid version should result in an error.");
        match result {
            Ok(_) => {
                assert!(false)
            },
            Err(msg) => {
                assert!(msg == "Version 2 not supported");
            }
        }
    }

    #[test]
    fn test_init_cluster() {

        let hexagon0 = HexagonMeta::new(Position{x: 1, y: 1, z: 1});
        let hexagon1 = HexagonMeta::new(Position{x: 3, y: 1, z: 1});
        let hexagon2 = HexagonMeta::new(Position{x: 4, y: 1, z: 1});
        let mut parsed_cluster = ClusterMeta::default();
        parsed_cluster.version = 1;
        parsed_cluster.settings.refractory_time = 2;
        parsed_cluster.settings.neuron_cooldown = 10000000.0f32;
        parsed_cluster.settings.max_connection_distance = 1;
        parsed_cluster.hexagons.insert(hexagon0.uuid, hexagon0.clone());
        parsed_cluster.hexagons.insert(hexagon1.uuid, hexagon1.clone());
        parsed_cluster.hexagons.insert(hexagon2.uuid, hexagon2.clone());
        parsed_cluster.axons.push(AxonMeta{from: Position{x: 1, y: 1, z: 1}, to: Position{x: 3, y: 1, z: 1}});
        parsed_cluster.inputs.push(InputMeta::new("input_hexagon".to_string(), Position{x: 1, y: 1, z: 1}));
        parsed_cluster.outputs.push(OutputMeta::new("output_hexagon".to_string(), Position{x: 4, y: 1, z: 1}, OutputType::PlainOutput));

        assert!(init_cluster(&mut parsed_cluster).is_ok());

        assert_eq!(parsed_cluster.settings.neuron_cooldown, 10000000.0);
        assert_eq!(parsed_cluster.settings.refractory_time, 2);
        assert_eq!(parsed_cluster.settings.max_connection_distance, 1);

        // test hexagons
        assert_eq!(parsed_cluster.hexagons.len(), 3);

        let parsed_hexagon0 = parsed_cluster.hexagons.get(&hexagon0.uuid).unwrap();
        let parsed_hexagon1 = parsed_cluster.hexagons.get(&hexagon1.uuid).unwrap();
        let parsed_hexagon2 = parsed_cluster.hexagons.get(&hexagon2.uuid).unwrap();

        assert_eq!(parsed_hexagon0.is_input, true);
        assert_eq!(parsed_hexagon0.is_output, false);
        // test neighbors of hexagon 0
        assert_eq!(parsed_hexagon0.neighbors[0], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[1], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[2], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[3], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[4], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[5], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[6], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[7], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[8], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[9], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[10], Uuid::nil());
        assert_eq!(parsed_hexagon0.neighbors[11], Uuid::nil());
        assert_eq!(parsed_hexagon0.axon_target, parsed_hexagon1.uuid);

        let mut success = true;
        for i in 0..NUMBER_OF_POSSIBLE_NEXT {
            success &= parsed_hexagon0.possible_hexagon_target_ids[i] == parsed_hexagon1.uuid
                    && parsed_hexagon0.possible_hexagon_target_ids[i] != parsed_hexagon0.uuid;
        }
        assert_eq!(success, true);


        assert_eq!(parsed_hexagon1.is_input, false);
        assert_eq!(parsed_hexagon1.is_output, false);
        // test neighbors of hexagon 1
        assert_eq!(parsed_hexagon1.neighbors[0], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[1], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[2], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[3], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[4], parsed_hexagon2.uuid);
        assert_eq!(parsed_hexagon1.neighbors[5], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[6], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[7], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[8], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[9], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[10], Uuid::nil());
        assert_eq!(parsed_hexagon1.neighbors[11], Uuid::nil());
        assert_eq!(parsed_hexagon1.axon_target, parsed_hexagon1.uuid);

        let mut success = true;
        for i in 0..NUMBER_OF_POSSIBLE_NEXT {
            success &= parsed_hexagon1.possible_hexagon_target_ids[i] == parsed_hexagon2.uuid
                    && parsed_hexagon1.possible_hexagon_target_ids[i] != parsed_hexagon1.uuid;
        }
        assert_eq!(success, true);

        assert_eq!(parsed_hexagon2.is_input, false);
        assert_eq!(parsed_hexagon2.is_output, true);
        // test neighbors of hexagon 2
        assert_eq!(parsed_hexagon2.neighbors[0], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[1], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[2], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[3], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[4], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[5], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[6], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[7], parsed_hexagon1.uuid);
        assert_eq!(parsed_hexagon2.neighbors[8], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[9], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[10], Uuid::nil());
        assert_eq!(parsed_hexagon2.neighbors[11], Uuid::nil());

        let mut success = true;
        for i in 0..NUMBER_OF_POSSIBLE_NEXT {
            success &= parsed_hexagon2.possible_hexagon_target_ids[i] == Uuid::nil();
        }
        assert_eq!(success, true);

    }
}
