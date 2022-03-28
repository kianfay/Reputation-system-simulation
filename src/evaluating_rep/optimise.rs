use crate::witness_rep::{
    simulation::SimulationConfig,
    quick_simulation::quick_simulation,
};
use crate::evaluating_rep::stats::*;

use anyhow::Result;
use std::ops::Range;
use rand::distributions::{Normal, Distribution};

/** The independant variable for application level variables.
 *  Deals only with non vector fields in SimulationConfig.
 */ 
/// A enum too hold the variable
#[derive(Clone, Debug)]
pub enum IndependantVarAppHollow {
    NumParticipants(Range<usize>),
    AverageProximity(Range<f32>),
    WitnessFloor(Range<usize>),
    Runs(Range<usize>),
}
/// A wrapper allowing us to deal polymorphically with App and Part
#[derive(Clone, Debug)]
pub struct IndependantVarApp {
    pub independant_var: IndependantVarAppHollow,
}

/** The independant variable for participant level variables.
 *  Needs to be separate from IndependantVarPartHollow because
 *  it relies on a variable (cur_mean) which is not present
 *  in SimulationConfig
 */
/// A enum too hold the variable
#[derive(Clone, Debug)]
pub enum IndependantVarPartHollow {
    /// Reliability(mean_range)
    Reliability,
    ReliabilityThreshold,
    DefaultReliability
}
/// A struct allowing for us to track the non sc variables
#[derive(Clone, Debug)]
pub struct IndependantVarPart {
    pub independant_var: IndependantVarPartHollow,
    pub current_mean: usize,
    pub current_std: usize,
    pub range: Range<usize>
}

/// The wrapper struct for the independant variable
#[derive(Clone, Debug)]
pub struct IndependantVar<C> {
    pub sc: SimulationConfig,
    pub independant_var: C
}

pub trait SCIterator {
    fn next(&mut self) -> Option<SimulationConfig>;
}

impl SCIterator for IndependantVar<IndependantVarApp> {
    fn next(&mut self) -> Option<SimulationConfig> {
        return get_next_config_app(&mut self.sc, &mut self.independant_var, 1);
    }
}

impl SCIterator for IndependantVar<IndependantVarPart> {
    fn next(&mut self) -> Option<SimulationConfig> {
        return get_next_config_part(&mut self.sc, &mut self.independant_var, 1);
    }
}


/* pub enum OptimalValue {
    UniqueF32(f32),
    UniqueUSize(usize),
    VecF32(Vec<f32>),
    VecUSize(Vec<usize>)
} */

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
pub async fn find_optimal<C>(
    ind_var: &mut C,
) -> Result<(usize, f32)> 
where C: SCIterator
{
    let mut results: Vec<(usize, f32)> = Vec::new();
    for i in 0.. {
        let next_sc = ind_var.next();
        match next_sc {
            None=> {break;},
            _   => {}
        };

        let (dir_name, ran_fully) = quick_simulation(next_sc.unwrap().clone(), false).await?;
        let rel_map = read_reliabilities(dir_name, false)?;
        let mse = run_avg_mean_squared_error(rel_map)?;
        if ran_fully {
            results.push((i, mse));
        } else {
            results.push((i, -1.0));
        }
    }

    println!("{:?}", &results);
    let optimal = results
        .into_iter()
        .reduce(|(x_accum, y_accum), (x_cur, y_cur)| {
            if y_cur < y_accum { (x_cur, y_cur) } else { (x_accum, y_accum) }
        }).unwrap();

    return Ok(optimal);
}

pub async fn find_optimal_two_fields<C1, C2>(
    ind_var_1: &mut C1,
    ind_var_2: &mut C2,
) -> Result<Vec<(usize, (usize, f32))>> 
where 
    C1: SCIterator,
    C2: SCIterator
{
    let mut results: Vec<(usize, (usize, f32))> = Vec::new();
    for i in 0.. {
        let next_sc_ = ind_var_1.next();
        match next_sc_ {
            None=> {break;},
            _   => {}
        };
        let next_sc = next_sc_.unwrap();

        let mut ind_var: IndependantVar<IndependantVarPart> = IndependantVar {
            sc: next_sc,
            independant_var: IndependantVarPart {
                independant_var: IndependantVarPartHollow::DefaultReliability,
                current_mean: 40,
                current_std: 2,
                range: 40..60
            }
        };

        let optimal = find_optimal(&mut ind_var).await?;
        results.push((i, optimal));
    }

    return Ok(results);
}

/// Updates the config to it's next state. Acts like an interator.
/// AverageProximity is a usize for type correctness, and represents the percentage
/// as a int in the range [0,100] instead of a float in the range [0,1]
pub fn get_next_config_app(
    sc: &mut SimulationConfig,
    ind_var: &mut IndependantVarApp,
    increments: usize
) -> Option<SimulationConfig> {

    let new_val: usize;
    match ind_var.clone().independant_var {
        IndependantVarAppHollow::NumParticipants(range)  => 
            {
                new_val = sc.num_participants + increments;
                if !range.contains(&new_val) {return None;}
                sc.num_participants = new_val;
            },
        IndependantVarAppHollow::AverageProximity(range) => 
            {
                let _new_val = sc.average_proximity + (increments as f32 / 100.0);
                if !range.contains(&_new_val) {return None;}
                sc.average_proximity = _new_val;
            },
        IndependantVarAppHollow::WitnessFloor(range)     => 
            {
                new_val = sc.witness_floor + increments;
                if !range.contains(&new_val) {return None;}
                sc.witness_floor = new_val;
            },
        IndependantVarAppHollow::Runs(range)             =>
            {
                new_val = sc.runs + increments;
                if !range.contains(&new_val) {return None;}
                sc.runs = new_val;
            },
/*         IndependantVar::Reliability(mean_range, cur_mean, std)  =>
            {
                new_val = cur_mean.clone() + increments;
                if !mean_range.contains(&new_val) {return (false, None);}

                let _new_val = new_val as f64 / 100.0;
                let normal = Normal::new(_new_val, (*std as f64) / 100.0);
                let reliabilities: Vec<f32> = normal
                    .sample_iter(&mut rand::thread_rng())
                    .take(sc.num_participants)
                    .map(|x| x as f32)
                    .collect();
                
                    println!("{:?}", reliabilities);
                
                sc.reliability = reliabilities;

                ind_var = &mut IndependantVar::Reliability(mean_range.clone(), new_val, std.clone());
            },
        _ => return (false, None) */
    }
    return Some(sc.clone());
}

pub fn get_next_config_part(
    sc: &mut SimulationConfig,
    ind_var: &mut IndependantVarPart,
    increments: usize
) -> Option<SimulationConfig> {
    // create updated mean
    let new_mean = ind_var.current_mean + increments;
    if !ind_var.range.contains(&new_mean) {return None}

    // generate new noramlly distributed vector
    let _new_mean = new_mean as f64 / 100.0;
    let normal = Normal::new(_new_mean, (ind_var.current_std as f64) / 100.0);
    let new_vec: Vec<f32> = normal
        .sample_iter(&mut rand::thread_rng())
        .take(sc.num_participants)
        .map(|x| x as f32)
        .collect();
    //println!("{:?}", reliabilities);

    // update the IndependantVarPart
    ind_var.current_mean = new_mean;

    // update the SimulationConfig
    match ind_var.clone().independant_var {
        IndependantVarPartHollow::Reliability  =>
        {   
            sc.reliability = new_vec;
        },
        IndependantVarPartHollow::ReliabilityThreshold  =>
        {   
            sc.reliability_threshold = new_vec;
        },
        IndependantVarPartHollow::DefaultReliability  =>
        {
            sc.default_reliability = new_vec;
        },
        _ => return None
    }

    // return the current SimulationConfig
    return Some(sc.clone());
}

#[test]
pub fn test_sc_iterator() {
    let sc = SimulationConfig {
        node_url: String::from(""),
        num_participants: 15,
        average_proximity: 0.5,
        witness_floor: 2,
        runs: 30,
        reliability: vec![1.0; 15],
        reliability_threshold: vec![1.0; 15],
        default_reliability: vec![1.0; 15],
        organizations: vec![1; 15]
    };
    let mut ind_var: IndependantVar<IndependantVarPart> = IndependantVar {
        sc: sc,
        independant_var: IndependantVarPart {
            independant_var: IndependantVarPartHollow::Reliability,
            current_mean: 40,
            current_std: 2,
            range: 40..60
        }
    };

    let mut rels: Vec<Vec<f32>> = Vec::new();
    for _ in 0..19 {
        //println!("SC: {:?}\n", ind_var.next().unwrap().reliability);
        rels.push(ind_var.next().unwrap().reliability);
    }
    let avg_rels: Vec<f32> = rels.iter().map(|v| v.iter().sum()).collect();

    // shallow test to see if the normal distribution is coming out right
    let ordered =   (avg_rels[0] < avg_rels[5]) && 
                    (avg_rels[5] < avg_rels[10]) && 
                    (avg_rels[10] < avg_rels[15]);

    assert_eq!(true, ordered);
}
