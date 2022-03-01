use crate::witness_rep::{
    simulation::SimulationConfig,
    quick_simulation::quick_simulation,
};
use crate::evaluating_rep::stats::*;

use anyhow::Result;
use std::ops::Range;
use rand::distributions::{Normal, Distribution};

// The range parameters specify the range of the independant variable
pub enum IndependantVar {
    
    // These represent independant variables which operate at the application level.
    NumParticipants(Range<usize>),
    AverageProximity(Range<f32>),
    WitnessFloor(Range<usize>),
    Runs(Range<usize>),

    // These represent independant variables which operate at the participant level (TODO)
    /// Reliability(mean_range, cur_mean, std)
    Reliability(Range<usize>, usize, usize),
    ReliabilityThreshold,
    DefaultReliability
}

pub enum OptimalValue {
    UniqueF32(f32),
    UniqueUSize(usize),
    VecF32(Vec<f32>),
    VecUSize(Vec<usize>)
}

/**
 * In the SimulationConfig struct:
 *  SimulationConfig {
        pub node_url: String,
        pub num_participants: usize,        **
        pub average_proximity: f32,         **
        pub witness_floor: usize,           **
        pub runs: usize,                    **
        pub reliability: Vec<f32>,          **  
        pub reliability_threshold: Vec<f32>,**  
        pub default_reliability: Vec<f32>,  **
        pub organizations: Vec<usize>,      
    }
 * the fields marked ** are the ones which can be independant variables
 */
pub async fn find_optimal(
    sc: &mut SimulationConfig,
    ind_var: IndependantVar,
    increments: usize
) -> Result<(usize, f32)> {
    let mut results: Vec<(usize, f32)> = Vec::new();
    loop {
        let (in_range, next_val) = get_next_config(sc, &ind_var, increments);
        if !in_range {break;}

        let dir_name = quick_simulation(sc.clone()).await?;
        let rel_map = read_reliabilities(dir_name, false)?;
        let mse = run_avg_mean_squared_error(rel_map)?;
        results.push((next_val.unwrap(), mse));
    }

    println!("{:?}", &results);
    let optimal = results
        .into_iter()
        .reduce(|(x_accum, y_accum), (x_cur, y_cur)| {
            if y_cur < y_accum { (x_cur, y_cur) } else { (x_accum, y_accum) }
        }).unwrap();

    return Ok(optimal);
}

/// Updates the config to it's next state. Acts like an interator.
/// AverageProximity is a usize for type correctness, and represents the percentage
/// as a int in the range [0,100] instead of a float in the range [0,1]
pub fn get_next_config(
    sc: &mut SimulationConfig,
    ind_var: &IndependantVar,
    increments: usize
) -> (bool, Option<usize>) {

    let new_val: usize;
    match ind_var {
        IndependantVar::NumParticipants(range)  => 
            {
                new_val = sc.num_participants + increments;
                if !range.contains(&new_val) {return (false, None);}
                sc.num_participants = new_val;
            },
        IndependantVar::AverageProximity(range) => 
            {
                let _new_val = sc.average_proximity + (increments as f32 / 100.0);
                if !range.contains(&_new_val) {return (false, None);}
                sc.average_proximity = _new_val;
                new_val = (_new_val * 100.0) as usize;
            },
        IndependantVar::WitnessFloor(range)     => 
            {
                new_val = sc.witness_floor + increments;
                if !range.contains(&new_val) {return (false, None);}
                sc.witness_floor = new_val;
            },
        IndependantVar::Runs(range)             =>
            {
                new_val = sc.runs + increments;
                if !range.contains(&new_val) {return (false, None);}
                sc.runs = new_val;
            },
        IndependantVar::Reliability(mean_range, cur_mean, std)  =>
            {
                new_val = cur_mean + increments;
                if !mean_range.contains(&new_val) {return (false, None);}

                let _new_val = new_val as f64 / 100.0;
                let normal = Normal::new(_new_val, (*std as f64) / 100.0);
                let reliabilities: Vec<f32> = normal
                    .sample_iter(&mut rand::thread_rng())
                    .take(sc.num_participants)
                    .map(|x| x as f32)
                    .collect();
                
                sc.reliability = reliabilities;
            },
        _ => return (false, None)
    }

    return (true, Some(new_val));
}