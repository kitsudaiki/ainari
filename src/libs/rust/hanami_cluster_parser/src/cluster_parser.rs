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

use pest::Parser;
use pest_derive::Parser;
use super::cluster_meta_structs::*;

#[derive(Parser)]
#[grammar = "cluster_parser.pest"]
pub struct ClusterParser;

pub fn parse_cluster_template(source_text: &str) -> Result<ClusterMeta, String> {
    let file_pair = ClusterParser::parse(Rule::file, source_text)
        .map_err(|e| format!("Parse error: {}", e))?
        .next().unwrap();

    let mut version: i32 = 0;
    let mut settings = Settings {
        neuron_cooldown: 10000000000.0,
        refractory_time: 1,
        max_connection_distance: 1,
    };
    let mut hexagons = Vec::new();
    let mut axons = Vec::new();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for section in file_pair.into_inner() {
        match section.as_rule() {
            Rule::version_r => {
                let number = section.into_inner().next().unwrap();
                version = number.as_str().parse().unwrap();
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
                for pos in section.into_inner().flat_map(|list| list.into_inner()) {
                    if pos.as_rule() == Rule::position {
                        hexagons.push(parse_position(pos));
                    }
                }
            }
            Rule::axons_r => {
                for axon_rule in section.into_inner() {
                    let mut inner = axon_rule.into_inner();
                    let from = parse_position(inner.next().unwrap());
                    let to = parse_position(inner.next().unwrap());
                    axons.push(Axon { from, to });
                }
            }
            Rule::inputs_r => {
                for input_rule in section.into_inner() {
                    let mut inner = input_rule.into_inner();
                    let name = inner.next().unwrap().as_str().to_string();
                    let pos = parse_position(inner.next().unwrap());
                    inputs.push(InputMeta { name, pos });
                }
            }
            Rule::outputs_r => {
                for out in section.into_inner() {
                    let mut inner = out.into_inner();
            
                    let name_str = inner.next().unwrap().as_str().to_string();
                    let pos = parse_position(inner.next().unwrap());
            
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
                        "plain" => OutputType::PLAIN_OUTPUT,
                        "float" => OutputType::FLOAT_OUTPUT,
                        "int"   => OutputType::INT_OUTPUT,
                        "bool"  => OutputType::BOOL_OUTPUT,
                        _       => OutputType::UNKNOWN_TYPE
                    };
            
                    // validation-check fo the type-field
                    if output_type == OutputType::UNKNOWN_TYPE {
                        return Err(format!("Invalid output extra value: '{}'", extra_field));
                    }

                    outputs.push(OutputMeta {
                        name: name_str,
                        pos: pos,
                        output_type: output_type,
                    })
                }
            }
            _ => {}
        }
    }

    Ok(ClusterMeta {
        version,
        settings,
        hexagons,
        axons,
        inputs,
        outputs,
    })
}

fn parse_position(pair: pest::iterators::Pair<Rule>) -> Position {
    let mut coords = pair.into_inner().map(|n| n.as_str().parse::<u32>().unwrap());
    Position(coords.next().unwrap(), coords.next().unwrap(), coords.next().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_version() {
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;";
            

        match parse_cluster_template(input) {
            Ok(parsed) => {
                assert_eq!(parsed.version, 42);
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
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 44;
            max_connection_distance: 43;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;";
            

        match parse_cluster_template(input) {
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
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;";
            

        match parse_cluster_template(input) {
            Ok(parsed) => {
                assert_eq!(parsed.hexagons.len(), 2);
                assert_eq!(parsed.hexagons[0], Position(1, 2, 3));
                assert_eq!(parsed.hexagons[1], Position(4, 5, 6));
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
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;";
            

        match parse_cluster_template(input) {
            Ok(parsed) => {
                assert_eq!(parsed.axons.len(), 2);
                assert_eq!(parsed.axons[0].from, Position(1, 1, 1));
                assert_eq!(parsed.axons[0].to, Position(2, 2, 2));
                assert_eq!(parsed.axons[1].from, Position(3, 3, 3));
                assert_eq!(parsed.axons[1].to, Position(4, 4, 4));
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
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3;
            key4: 4,4,4;";
            
        match parse_cluster_template(input) {
            Ok(parsed) => {
                assert_eq!(parsed.inputs.len(), 2);
                assert_eq!(parsed.inputs[0].name, "key1");
                assert_eq!(parsed.inputs[0].pos, Position(1, 1, 1));
                assert_eq!(parsed.inputs[1].name, "key2");
                assert_eq!(parsed.inputs[1].pos, Position(2, 2, 2));
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
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3;
            key4: 4,4,4;";

        match parse_cluster_template(input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].pos, Position(3, 3, 3));
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].pos, Position(4, 4, 4));
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
        let input = "version: 42 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,2,3; 
            4,5,6; 
        axons: 
            1,1,1 -> 2,2,2; 
            3,3,3 -> 4,4,4; 
        inputs: 
            key1: 1,1,1; 
            key2: 2,2,2; 
        outputs: 
            key3: 3,3,3 (float);
            key4: 4,4,4;";

        match parse_cluster_template(input) {
            Ok(parsed) => {
                assert_eq!(parsed.outputs.len(), 2);
                assert_eq!(parsed.outputs[0].name, "key3");
                assert_eq!(parsed.outputs[0].pos, Position(3, 3, 3));
                assert_eq!(parsed.outputs[0].output_type, OutputType::FLOAT_OUTPUT);
                assert_eq!(parsed.outputs[1].name, "key4");
                assert_eq!(parsed.outputs[1].pos, Position(4, 4, 4));
                assert_eq!(parsed.outputs[1].output_type, OutputType::PLAIN_OUTPUT);
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
        let input = "";
        let result = parse_cluster_template(input);
        assert!(result.is_err(), "Empty input should result in an error.");
    }
    
    #[test]
    fn test_invalid_input() {
        let input = "version: 42 
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
        let result = parse_cluster_template(input);
        assert!(result.is_err(), "Invalid input should result in an error.");
    }
}
