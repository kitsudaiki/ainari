// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
use rand::RngExt;
use uuid::Uuid;

use super::model_meta_structs::*;

use ainari_common::constants::*;
use ainari_common::enums::*;
use ainari_common::error::AinariError;
use ainari_common::functions::*;
use ainari_common::objects::*;

/// Parser for Ainari model templates using Pest parser combinator framework.
#[derive(Parser)]
#[grammar = "model_parser.pest"]
pub struct ModelParser;

/// Parses a model template from source text and returns a ModelMeta structure.
///
/// # Arguments
///
/// * `name` - The name of the model to be created
/// * `source_text` - The source text containing the model template
///
/// # Returns
///
/// * `Result<ModelMeta, AinariError>` - The parsed model or an error if parsing fails
///
/// # Errors
///
/// Returns an AinariError if:
/// * The template version is not supported
/// * The input format is invalid
/// * Any required fields are missing or malformed
pub fn parse_model_template(name: &str, source_text: &str) -> Result<ModelMeta, AinariError> {
    let file_pair = ModelParser::parse(Rule::file, source_text)
        .map_err(|e| {
            AinariError::InvalidInput(format!("Failed to parse model-template with error: {e}",))
        })?
        .next()
        .unwrap();

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
                    return Err(AinariError::InvalidInput(msg));
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
                        hexagons.insert(hexagon_meta.uuid, hexagon_meta);
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
                        "int" => OutputType::IntOutput,
                        "bool" => OutputType::BoolOutput,
                        _ => {
                            let msg = format!("Invalid output extra value: '{extra_field}'");
                            return Err(AinariError::InvalidInput(msg));
                        }
                    };

                    outputs.push(OutputMeta::new(name_str, position, output_type));
                }
            }
            _ => {}
        }
    }

    let mut parsed_model = ModelMeta {
        uuid: Uuid::nil(),
        name: name.to_owned(),
        version,
        settings,
        hexagons,
        axons,
        inputs,
        outputs,
    };

    init_model(&mut parsed_model)?;

    Ok(parsed_model)
}

/// Parses a position string from a pest iterator pair into a Position struct.
///
/// # Arguments
///
/// * `pair` - A pest iterator pair containing the position coordinates
///
/// # Returns
///
/// A Position struct with x, y, and z coordinates parsed from the input string.
///
/// # Panics
///
/// This function will panic if the input cannot be parsed into coordinates.
fn parse_position(pair: pest::iterators::Pair<Rule>) -> Position {
    let mut coords = pair
        .into_inner()
        .map(|n| n.as_str().parse::<u32>().unwrap());
    Position {
        x: coords.next().unwrap(),
        y: coords.next().unwrap(),
        z: coords.next().unwrap(),
    }
}

/// Initializes the complete model by setting up all necessary connections and properties.
///
/// This function coordinates the initialization of all model components:
/// * Hexagons and their connections
/// * Axons
/// * Inputs
/// * Outputs
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta to be initialized
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if initialization succeeds, Err if any step fails
fn init_model(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    initialize_hexagons(parsed_model)?;
    update_axons(parsed_model)?;
    initialize_inputs(parsed_model)?;
    initialize_outputs(parsed_model)?;

    Ok(())
}

/// Searches for a hexagon at a specific position with a given type.
///
/// This function looks through all hexagons to find one at the specified position.
/// It's used to validate positions of inputs, outputs, and axon connections.
///
/// # Arguments
///
/// * `hexagons_meta` - A reference to the HashMap of hexagon metadata
/// * `position` - The position to search for
/// * `type_name` - A string describing the type of element being positioned (for error messages)
///
/// # Returns
///
/// * `Result<Uuid, AinariError>` - The UUID of the found hexagon or an error if not found
fn search_hexagon(
    hexagons_meta: &HashMap<Uuid, HexagonMeta>,
    position: &Position,
    type_name: &str,
) -> Result<Uuid, AinariError> {
    for h in hexagons_meta.values() {
        if h.positon == *position {
            return Ok(h.uuid);
        }
    }

    let msg = format!("Invalid {type_name} position: {position}");
    Err(AinariError::InvalidInput(msg))
}

/// Initializes the input connections for the model.
///
/// This function:
/// 1. Validates each input position
/// 2. Marks the corresponding hexagon as an input
/// 3. Sets the hexagon UUID in the input metadata
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being initialized
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if initialization succeeds, Err if any input is invalid
fn initialize_inputs(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    for input_meta in parsed_model.inputs.iter_mut() {
        let hexagon_uuid = search_hexagon(&parsed_model.hexagons, &input_meta.position, "input")?;
        if let Some(obj) = parsed_model.hexagons.get_mut(&hexagon_uuid) {
            obj.is_input = true;
            input_meta.hexagon_uuid = hexagon_uuid;
        } else {
            return Err(AinariError::InternalError(format!(
                "Can not find input-hexagon with ID {hexagon_uuid}, even it should exist."
            )));
        }
    }
    Ok(())
}

/// Initializes the output connections for the model.
///
/// This function:
/// 1. Validates each output position
/// 2. Marks the corresponding hexagon as an output
/// 3. Sets the hexagon UUID and name in the output metadata
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being initialized
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if initialization succeeds, Err if any output is invalid
fn initialize_outputs(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    for output_meta in parsed_model.outputs.iter_mut() {
        let hexagon_uuid = search_hexagon(&parsed_model.hexagons, &output_meta.position, "output")?;
        if let Some(obj) = parsed_model.hexagons.get_mut(&hexagon_uuid) {
            obj.is_output = true;
            obj.name = output_meta.name.clone();
            output_meta.hexagon_uuid = hexagon_uuid;
        } else {
            return Err(AinariError::InternalError(format!(
                "Can not find output-hexagon with ID {hexagon_uuid}, even it should exist."
            )));
        }
    }
    Ok(())
}

/// Initializes the hexagon structure of the model.
///
/// This function coordinates the following steps:
/// 1. Updates axon connections
/// 2. Connects all hexagons to their neighbors
/// 3. Initializes the target hexagon list for each hexagon
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being initialized
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if initialization succeeds, Err if any step fails
fn initialize_hexagons(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    update_axons(parsed_model)?;
    connect_all_hexagons(parsed_model)?;
    initialize_target_hexagon_list(parsed_model)?;

    Ok(())
}

/// Updates the axon connections for all hexagons.
///
/// This function:
/// 1. Validates each axon connection
/// 2. Sets the axon_target for each source hexagon
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being updated
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if all axons are valid, Err if any axon connection is invalid
fn update_axons(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    for axon in parsed_model.axons.clone() {
        let from_id = search_hexagon(&parsed_model.hexagons, &axon.from, "axon with source")?;
        let target_id = search_hexagon(&parsed_model.hexagons, &axon.to, "axon with target")?;

        if let Some(obj) = parsed_model.hexagons.get_mut(&from_id) {
            obj.axon_target = target_id;
        }
    }

    Ok(())
}

/// Connects a single hexagon to its neighbor in a specified direction.
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being updated
/// * `hexagon_copy` - A reference to the complete set of hexagons
/// * `source_id` - The UUID of the source hexagon
/// * `source_pos` - The position of the source hexagon
/// * `side` - The side number (0-11) representing the direction to check for neighbors
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the connection is successful or no neighbor exists
fn connect_hexagon(
    parsed_model: &mut ModelMeta,
    hexagon_copy: &HashMap<Uuid, HexagonMeta>,
    source_id: &Uuid,
    source_pos: &Position,
    side: usize,
) -> Result<(), AinariError> {
    let next = get_neighbor_pos(source_pos, side);
    if next.is_valid() {
        for (target_id, hexagon) in hexagon_copy.iter() {
            let target_pos = &hexagon.positon;
            if *target_pos == next {
                if let Some(source_obj) = parsed_model.hexagons.get_mut(&source_id.clone()) {
                    source_obj.neighbors[side] = *target_id;
                }
                if let Some(target_obj) = parsed_model.hexagons.get_mut(target_id) {
                    target_obj.neighbors[11 - side] = *source_id;
                }
            }
        }
    }

    Ok(())
}

/// Connects all hexagons to their neighboring hexagons in all 12 possible directions.
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being updated
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if all connections are successful
fn connect_all_hexagons(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    let hexagon_copy = parsed_model.hexagons.clone();
    for (source_id, source_hexagon) in hexagon_copy.iter() {
        let source_pos = &source_hexagon.positon;
        for side in 0..12 {
            connect_hexagon(parsed_model, &hexagon_copy, source_id, source_pos, side)?;
        }
    }

    Ok(())
}

/// Navigates through the hexagon grid to find a target hexagon.
///
/// This function implements a randomized pathfinding algorithm that:
/// 1. Moves from the current hexagon to neighboring hexagons
/// 2. Limits the path length based on the model's max_connection_distance
/// 3. Has a 50% chance to stop at each step (except when at the source hexagon)
///
/// # Arguments
///
/// * `hexagons_static_copy` - A reference to the complete set of hexagons
/// * `current_hexagon` - The current hexagon being examined
/// * `source_id` - The UUID of the source hexagon
/// * `max_path_length` - A mutable reference to the remaining path length
///
/// # Returns
///
/// * `Result<Uuid, AinariError>` - The UUID of the target hexagon or an error if navigation fails
fn go_to_next_hexagon(
    hexagons_static_copy: &HashMap<Uuid, HexagonMeta>,
    current_hexagon: &HexagonMeta,
    source_id: &Uuid,
    max_path_length: &mut i32,
) -> Result<Uuid, AinariError> {
    // check path-length to not go too far
    *max_path_length -= 1;
    if *max_path_length < 0 {
        return Ok(current_hexagon.uuid);
    }

    let chance_for_next = 0.5f32; // TODO: make hard-coded value configurable
    let num = rand::rng().random_range(0..1000) as f32;
    if (chance_for_next * 1000.0f32 < num) && source_id != &current_hexagon.uuid {
        return Ok(current_hexagon.uuid);
    }

    let mut available_next: Vec<Uuid> = vec![];
    let possible_next_sides = [9, 3, 1, 4, 11, 5, 2];
    for side in possible_next_sides {
        let next_hexagon_id = current_hexagon.neighbors[side];
        if next_hexagon_id != Uuid::nil() {
            available_next.push(next_hexagon_id);
        }
    }

    if available_next.is_empty() {
        return Ok(current_hexagon.uuid);
    }

    let next_select = rand::rng().random_range(0..available_next.len());
    let selected_next_id = &available_next[next_select];

    if let Some(obj) = hexagons_static_copy.get(selected_next_id) {
        go_to_next_hexagon(hexagons_static_copy, obj, source_id, max_path_length)
    } else {
        Ok(current_hexagon.uuid)
    }
}

/// Initializes the list of possible target hexagons for each hexagon in the model.
///
/// This function:
/// 1. For each hexagon, finds possible target hexagons based on its axon_target
/// 2. Considers the max_connection_distance setting
/// 3. Gives hexagons with different axon_targets an advantage in selection
///
/// # Arguments
///
/// * `parsed_model` - A mutable reference to the ModelMeta being initialized
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if initialization succeeds, Err if any step fails
fn initialize_target_hexagon_list(parsed_model: &mut ModelMeta) -> Result<(), AinariError> {
    let hexagons_static_copy = parsed_model.hexagons.clone();
    for (source_uuid, source_hexagon) in parsed_model.hexagons.iter_mut() {
        for counter in 0..NUMBER_OF_POSSIBLE_NEXT {
            let mut max_path_length = parsed_model.settings.max_connection_distance as i32;

            // hexagons with a different axon-target have an advantage, so they reduced by one step to be equal to other hexagons
            if source_uuid != &source_hexagon.axon_target {
                max_path_length -= 1;
            }

            let base_hexagon = &hexagons_static_copy
                .get(&source_hexagon.axon_target)
                .unwrap();
            let target_hexagon_id = go_to_next_hexagon(
                &hexagons_static_copy,
                base_hexagon,
                source_uuid,
                &mut max_path_length,
            )?;

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
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.version, 1);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_settings() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.settings.neuron_cooldown, 1000000000.0);
                assert_eq!(parsed.settings.refractory_time, 44);
                assert_eq!(parsed.settings.max_connection_distance, 43);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_tests() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.hexagons.len(), 2);

                if let Some(value) = parsed.hexagons.values().next() {
                    let valid = value.positon == Position { x: 1, y: 2, z: 3 }
                        || value.positon == Position { x: 4, y: 5, z: 6 };
                    assert!(valid);
                } else {
                    assert_eq!(false, true);
                }
                if let Some(value) = parsed.hexagons.values().nth(1) {
                    let valid = value.positon == Position { x: 1, y: 2, z: 3 }
                        || value.positon == Position { x: 4, y: 5, z: 6 };
                    assert!(valid);
                } else {
                    assert_eq!(false, true);
                }
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_axons() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.axons.len(), 2);
                assert_eq!(parsed.axons[0].from, Position { x: 1, y: 2, z: 3 });
                assert_eq!(parsed.axons[0].to, Position { x: 2, y: 2, z: 2 });
                assert_eq!(parsed.axons[1].from, Position { x: 2, y: 2, z: 2 });
                assert_eq!(parsed.axons[1].to, Position { x: 4, y: 5, z: 6 });
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_inputs() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.inputs.len(), 2);
                assert_eq!(parsed.inputs[0].name, "key1");
                assert_eq!(parsed.inputs[0].position, Position { x: 1, y: 1, z: 1 });
                assert_eq!(parsed.inputs[1].name, "key2");
                assert_eq!(parsed.inputs[1].position, Position { x: 2, y: 2, z: 2 });
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_outputs() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].position, Position { x: 3, y: 3, z: 3 });
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].position, Position { x: 4, y: 4, z: 4 });
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_outputs_with_extra() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].position, Position { x: 3, y: 3, z: 3 });
                assert_eq!(parsed.outputs[0].output_type, OutputType::FloatOutput);
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].position, Position { x: 4, y: 4, z: 4 });
                assert_eq!(parsed.outputs[1].output_type, OutputType::PlainOutput);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_parse_outputs_without_axons() {
        let name = "test-model".to_string();
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

        match parse_model_template(&name, input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].position, Position { x: 3, y: 3, z: 3 });
                assert_eq!(parsed.outputs[0].output_type, OutputType::FloatOutput);
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].position, Position { x: 4, y: 4, z: 4 });
                assert_eq!(parsed.outputs[1].output_type, OutputType::PlainOutput);
            }
            Err(e) => {
                eprintln!("❌ Parsing Error: {e}");
                // should always fail here
                assert_eq!(false, true);
            }
        }
    }

    #[test]
    fn test_empty_input() {
        let name = "test-model".to_string();
        let input = "";
        let result = parse_model_template(&name, input);
        assert!(result.is_err(), "Empty input should result in an error.");
    }

    #[test]
    fn test_invalid_input() {
        let name = "test-model".to_string();
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
        let result = parse_model_template(&name, input);
        assert!(result.is_err(), "Invalid input should result in an error.");
    }

    #[test]
    fn test_invalid_version() {
        let name = "test-model".to_string();
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
        let result = parse_model_template(&name, input);
        assert!(
            result.is_err(),
            "Invalid version should result in an error."
        );
        match result {
            Ok(_) => {
                panic!();
            }
            Err(msg) => {
                assert!(msg == "Version 2 not supported");
            }
        }
    }

    #[test]
    fn test_init_model() {
        let hexagon0 = HexagonMeta::new(Position { x: 1, y: 1, z: 1 });
        let hexagon1 = HexagonMeta::new(Position { x: 3, y: 1, z: 1 });
        let hexagon2 = HexagonMeta::new(Position { x: 4, y: 1, z: 1 });
        let mut parsed_model = ModelMeta {
            version: 1,
            ..Default::default()
        };
        parsed_model.settings.refractory_time = 2;
        parsed_model.settings.neuron_cooldown = 10000000.0f32;
        parsed_model.settings.max_connection_distance = 1;
        parsed_model
            .hexagons
            .insert(hexagon0.uuid, hexagon0.clone());
        parsed_model
            .hexagons
            .insert(hexagon1.uuid, hexagon1.clone());
        parsed_model
            .hexagons
            .insert(hexagon2.uuid, hexagon2.clone());
        parsed_model.axons.push(AxonMeta {
            from: Position { x: 1, y: 1, z: 1 },
            to: Position { x: 3, y: 1, z: 1 },
        });
        parsed_model.inputs.push(InputMeta::new(
            "input_hexagon".to_string(),
            Position { x: 1, y: 1, z: 1 },
        ));
        parsed_model.outputs.push(OutputMeta::new(
            "output_hexagon".to_string(),
            Position { x: 4, y: 1, z: 1 },
            OutputType::PlainOutput,
        ));

        assert!(init_model(&mut parsed_model).is_ok());

        assert_eq!(parsed_model.settings.neuron_cooldown, 10000000.0);
        assert_eq!(parsed_model.settings.refractory_time, 2);
        assert_eq!(parsed_model.settings.max_connection_distance, 1);

        // test hexagons
        assert_eq!(parsed_model.hexagons.len(), 3);

        let parsed_hexagon0 = parsed_model.hexagons.get(&hexagon0.uuid).unwrap();
        let parsed_hexagon1 = parsed_model.hexagons.get(&hexagon1.uuid).unwrap();
        let parsed_hexagon2 = parsed_model.hexagons.get(&hexagon2.uuid).unwrap();

        assert!(parsed_hexagon0.is_input);
        assert!(!parsed_hexagon0.is_output);
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
        assert!(success);

        assert!(!parsed_hexagon1.is_input);
        assert!(!parsed_hexagon1.is_output);
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
        assert!(success);

        assert!(!parsed_hexagon2.is_input);
        assert!(parsed_hexagon2.is_output);
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
        assert!(success);
    }
}
