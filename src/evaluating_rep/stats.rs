use anyhow::Result;
use std::fs;
use rustlearn::{
    array::dense::Array,
    metrics::mean_squared_error
};
use std::collections::HashMap;

pub type TrueReliability    = f32;
pub type EstimReliabilities = Vec<f32>;
pub type Reliability        = (TrueReliability, EstimReliabilities);
pub type ReputationMap     = HashMap<String, Reliability>;

pub fn run_avg_mean_squared_error(rel_map: ReputationMap) -> Result<f32> {
    // average out the the estimates
    let mut true_rels: Vec<f32> = Vec::new();
    let mut estm_rels: Vec<f32> = Vec::new();
    for (_, (true_rel, est_rels)) in rel_map{
        // insert the true reliability score
        true_rels.push(true_rel);
        
        // insert the average estimated reliability score
        let avg_estimates: f32 = est_rels.iter().sum::<f32>() / est_rels.len() as f32;
        estm_rels.push(avg_estimates);
    }

    let true_arr: Array = Array::from(true_rels.clone());
    let estm_arr: Array = Array::from(estm_rels.clone());
/*     println!("{:?}", true_rels);
    println!("{:?}", estm_rels); */

    let mse = mean_squared_error(&true_arr, &estm_arr);
    return Ok(mse)
}

pub fn read_reliabilities(dir_name: String, use_arg: bool) -> Result<ReputationMap> {
    // read the data from file
    let file_name_start: String;
    let file_name_end: String;
    if use_arg {
        file_name_start = format!("./runs/{}/start_reliability.txt", dir_name);
        file_name_end = format!("./runs/{}/reliability_maps.txt", dir_name);
    } else {
        file_name_start = format!("{}/start_reliability.txt", dir_name);
        file_name_end = format!("{}/reliability_maps.txt", dir_name);
    }
    println!("{}", file_name_start);
    let start_rels = fs::read_to_string(file_name_start)?;
    let end_rels = fs::read_to_string(file_name_end)?;

    // insert the true reliabilities into the ReputationMap
    let mut rel_map = ReputationMap::new();
    for line in start_rels.split('\n') {
        // extract the pubkey and the true reliability
        let line_option = get_line_info(line);
            if let None = line_option {
                continue;
            }
        let (pubkey, true_rel) = line_option.unwrap();

        // instantiate the relevant types
        let est_rels = EstimReliabilities::new();
        
        // create a blank entry for the particpant
        rel_map.entry(pubkey).or_insert((true_rel, est_rels));
    }

    // inserting the estimated reliabilities into the ReputationMaps
    for paragraph in end_rels.split("\n\n\n") {
        let mut lines = paragraph.split('\n');

        // remove the pubkey from the paragraph (not needed currently)
        let _ = lines.next().unwrap();
        
        // for each line in the paragraph, extract the estimate and insert
        for line in lines {
            let line_option = get_line_info(line);
            if let None = line_option {
                continue;
            }
            let (est_part_pk, est_rel) = line_option.unwrap();

            // get the mapping for the est_part_pk participant, and add new estimate
            let (true_rel, mut est_rels) = rel_map.remove(&est_part_pk).unwrap();
            est_rels.push(est_rel);

            // reinsert into hashmap
            rel_map.insert(est_part_pk, (true_rel, est_rels));
        }
    }
    
    return Ok(rel_map);
}

pub fn get_line_info(line: &str) -> Option<(String, f32)> {
    if line == "" {
        return None;
    }
    let mut split = line.split(": ");
    let str = split.next().unwrap();
    let rel: f32 = split.next().unwrap().parse().unwrap();
    return Some((String::from(str), rel));
}