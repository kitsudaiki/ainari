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

use rand::Rng;
use rand::seq::IteratorRandom;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_common::constants::*;
use ainari_common::error::AinariError;

use crate::core::blocks::axons::AxonSection;
use crate::core::blocks::block_trait::Block;
use crate::core::blocks::core_block::*;
use crate::core::blocks::output_block::*;
use crate::core::cluster_handler::*;

#[derive(Default, Debug)]
struct TargetInformation {
    hexagon_uuid: Uuid,
    is_output: bool,
    output_hexagon_name: String,
}

pub fn connect_to_new_target(axon_section: &mut AxonSection) -> Result<(), AinariError> {
    check_axon_setion(axon_section)?;

    let target_information = get_target_hexagon(axon_section)?;

    let source_block;
    let cluster_settings;
    let selected_block_option;

    {
        let cluster_handler = CLUSTER_HANDLER.read().unwrap();

        // get source-block
        source_block = cluster_handler.get_block(
            &axon_section.cluster_uuid,
            &axon_section.source_hexagon_uuid,
            &axon_section.source_block_uuid,
        )?;

        let cluster_link = cluster_handler.get_cluster(&axon_section.cluster_uuid)?;
        cluster_settings = cluster_link.cluster_meta.settings.clone();
        let binding = cluster_link.hexagon_data.read().unwrap();
        let target_hexagon_link = if let Some(h) = binding.get(&target_information.hexagon_uuid) {
            h.lock().unwrap()
        } else {
            let msg = format!(
                "Hexagon with uuid '{}' not found.",
                target_information.hexagon_uuid
            );
            return Err(AinariError::InvalidInput(msg));
        };

        match random_value(&target_hexagon_link.blocks) {
            Some(value) => {
                selected_block_option = Some(value.clone());
            }
            None => {
                selected_block_option = None;
            }
        }
    }

    // check if the reandome selected block is available
    if let Some(selected_block_mutex) = selected_block_option {
        let mut selected_block = selected_block_mutex.lock().unwrap();
        if selected_block.get_free_input(axon_section) {
            axon_section.target_block = Some(selected_block_mutex.clone());
            axon_section.source_block = Some(source_block);
            return Ok(());
        }
    }

    // create new block
    if target_information.is_output {
        let mut cluster_handler = CLUSTER_HANDLER.write().unwrap();
        let output_block_mutex = Arc::new(Mutex::new(OutputBlock::new(
            &target_information.hexagon_uuid,
            &axon_section.cluster_uuid,
            &target_information.output_hexagon_name,
        )));
        cluster_handler.add_output_block(&output_block_mutex)?;
        drop(cluster_handler);
        let mut output_block = output_block_mutex.lock().unwrap();
        if output_block.get_free_input(axon_section) {
            axon_section.target_block = Some(output_block_mutex.clone());
            axon_section.source_block = Some(source_block);
            return Ok(());
        }
    } else {
        let mut cluster_handler = CLUSTER_HANDLER.write().unwrap();
        let core_block_mutex = Arc::new(Mutex::new(CoreBlock::new(
            &target_information.hexagon_uuid,
            &axon_section.cluster_uuid,
            &cluster_settings,
        )));
        cluster_handler.add_core_block(&core_block_mutex)?;
        drop(cluster_handler);
        let mut core_block = core_block_mutex.lock().unwrap();
        if core_block.get_free_input(axon_section) {
            axon_section.target_block = Some(core_block_mutex.clone());
            axon_section.source_block = Some(source_block);
            return Ok(());
        }
    }

    let msg = format!(
        "Failed to connect block with uuid '{}' with a target.",
        axon_section.source_block_uuid
    );
    Err(AinariError::Error(msg))
}

fn random_value<K, V>(map: &HashMap<K, V>) -> Option<&V>
where
    K: std::hash::Hash + Eq,
{
    let mut rng = rand::rng();
    map.values().choose(&mut rng)
}

fn check_axon_setion(axon_section: &mut AxonSection) -> Result<(), AinariError> {
    // pre-check
    if axon_section.cluster_uuid == Uuid::nil()
        || axon_section.source_block_uuid == Uuid::nil()
        || axon_section.source_hexagon_uuid == Uuid::nil()
        || axon_section.source_pos == UNINIT_STATE_8
    {
        let msg = "Got invalid Axon-setion.".to_string();
        return Err(AinariError::Error(msg));
    }

    Ok(())
}

fn get_target_hexagon(axon_section: &mut AxonSection) -> Result<TargetInformation, AinariError> {
    let mut cluster_handler = CLUSTER_HANDLER.write().unwrap();
    let mut target_information = TargetInformation::default();
    let cluster_link = cluster_handler.get_cluster_mut(&axon_section.cluster_uuid)?;

    // get the uuid of the target-hexagon
    if let Some(source_hexagon_meta) = cluster_link
        .cluster_meta
        .hexagons
        .get(&axon_section.source_hexagon_uuid)
    {
        let random_pos = rand::rng().random_range(0..NUMBER_OF_POSSIBLE_NEXT) as usize;
        target_information.hexagon_uuid =
            source_hexagon_meta.possible_hexagon_target_ids[random_pos];
    } else {
        let msg = format!(
            "Hexagon with uuid '{}' not found in cluster-meta.",
            axon_section.source_hexagon_uuid
        );
        return Err(AinariError::InvalidInput(msg));
    };

    if let Some(target_hexagon_meta) = cluster_link
        .cluster_meta
        .hexagons
        .get(&target_information.hexagon_uuid)
    {
        target_information.is_output = target_hexagon_meta.is_output;
        target_information.output_hexagon_name = target_hexagon_meta.name.clone();

        // input-hexagons are not allowed to be a target
        if target_hexagon_meta.is_input {
            let msg = format!(
                "Hexagon with uuid '{}' is input-hexagon and can not be used as output.",
                target_information.hexagon_uuid
            );
            return Err(AinariError::InvalidInput(msg));
        }
    } else {
        let msg = format!(
            "Hexagon with uuid '{}' not found in cluster-meta.",
            target_information.hexagon_uuid
        );
        return Err(AinariError::InvalidInput(msg));
    };

    // add hexagon if necessary
    let mut hexagon_data = cluster_link.hexagon_data.write().unwrap();
    hexagon_data
        .entry(target_information.hexagon_uuid)
        .or_insert_with(|| Arc::new(Mutex::new(HexagonData::new())));

    Ok(target_information)
}

#[cfg(test)]
mod tests {
    use crate::core::blocks::input_block::*;
    use crate::core::processing::finish_counter::FinishCounter;
    use crate::core::processing::output_buffer::*;

    use ainari_cluster_parser::cluster_meta_structs::Settings;
    use ainari_common::enums::*;

    use super::*;

    #[test]
    fn test_resize() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let cluster_uuid = Uuid::new_v4();
        let hexagon_uuid0;
        let hexagon_uuid1;
        let cluster_name = "test_cluster".to_string();
        let input_name = "test_input".to_string();
        let output_name = "test_output".to_string();
        let template = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,1,1; 
            2,2,2; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            key1: 1,1,1; 
        outputs: 
            key2: 2,2,2;"
            .to_string();

        let mut root_handler = CLUSTER_HANDLER.write().unwrap();
        root_handler.clusters.clear();
        let _ = root_handler.init_new_cluster(&cluster_uuid, &cluster_name, template);

        {
            let cluster = root_handler.clusters.get(&cluster_uuid).unwrap();
            if cluster
                .cluster_meta
                .hexagons
                .values()
                .next()
                .unwrap()
                .is_input
            {
                hexagon_uuid0 = *cluster.cluster_meta.hexagons.keys().next().unwrap();
                hexagon_uuid1 = *cluster.cluster_meta.hexagons.keys().nth(1).unwrap();
            } else {
                hexagon_uuid1 = *cluster.cluster_meta.hexagons.keys().next().unwrap();
                hexagon_uuid0 = *cluster.cluster_meta.hexagons.keys().nth(1).unwrap();
            }
        }

        // prepare new blocks
        let settings = Settings::default();
        let core_block_mutex = Arc::new(Mutex::new(CoreBlock::new(
            &hexagon_uuid0,
            &cluster_uuid,
            &settings,
        )));
        let input_block_mutex = Arc::new(Mutex::new(InputBlock::new(
            &input_name,
            &hexagon_uuid0,
            &cluster_uuid,
            &finish_counter,
        )));
        let output_block_mutex = Arc::new(Mutex::new(OutputBlock::new(
            &hexagon_uuid1,
            &cluster_uuid,
            &output_name,
        )));
        let output_buffer_mutex = Arc::new(Mutex::new(OutputBuffer::new(
            &output_name,
            &hexagon_uuid1,
            &cluster_uuid,
            &OutputType::PlainOutput,
            &finish_counter,
        )));

        // add blocks to cluster
        let _ = root_handler.add_core_block(&core_block_mutex);
        let _ = root_handler.add_input_block(&input_block_mutex);
        let _ = root_handler.add_output_block(&output_block_mutex);
        let _ = root_handler.add_output_buffer(&output_buffer_mutex);
        drop(root_handler);

        let mut test_section = AxonSection::default();
        let core_block = core_block_mutex.lock().unwrap();
        test_section.source_block_uuid = core_block.uuid;
        test_section.source_hexagon_uuid = core_block.hexagon_uuid;
        test_section.cluster_uuid = core_block.cluster_uuid;
        test_section.source_pos = 0;

        match connect_to_new_target(&mut test_section) {
            Ok(()) => {}
            Err(e) => {
                println!("{e}");
                panic!();
            }
        }

        assert_eq!(test_section.source_block_uuid, core_block.uuid);
        assert_eq!(test_section.source_hexagon_uuid, core_block.hexagon_uuid);
        assert_eq!(test_section.cluster_uuid, core_block.cluster_uuid);
        assert_eq!(test_section.source_pos, 0);
        assert_eq!(test_section.target_hexagon_uuid, hexagon_uuid1);
        assert!(test_section.source_block.is_some());
        assert!(test_section.target_block.is_some());
    }
}
