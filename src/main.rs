use anyhow::Result;
use std::env;
use evaluating_rep::optimise::*;

mod witness_rep;
mod evaluating_rep;

#[tokio::main]
async fn main() -> Result<()> {
    let url = "http://0.0.0.0:14265";

/*     let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_participants: 15,
        average_proximity: 0.5,
        witness_floor: 2,
        runs: 30,
        reliability: vec![0.8; 15],
        reliability_threshold: vec![0.1; 15],
        default_reliability: vec![0.5; 15],
        organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2]
    };

    let mut ind_var_1: IndependantVar<IndependantVarPart> = IndependantVar {
        sc: sc.clone(),
        independant_var: IndependantVarPart {
            independant_var: IndependantVarPartHollow::ReliabilityThreshold,
            current_mean: 1,
            current_std: 10,
            range: 1..98
        }
    };
    let mut ind_var_2: IndependantVar<IndependantVarPart> = IndependantVar {
        sc: sc,
        independant_var: IndependantVarPart {
            independant_var: IndependantVarPartHollow::DefaultReliability,
            current_mean: 1,
            current_std: 10,
            range: 1..98
        }
    };
    let optimal = find_optimal_two_fields(&mut ind_var_1, &mut ind_var_2).await?;
    println!("{:?}", optimal); */

/*     let mut ind_var: IndependantVar<IndependantVarPart> = IndependantVar {
        sc: sc,
        independant_var: IndependantVarPart {
            independant_var: IndependantVarPartHollow::DefaultReliability,
            current_mean: 40,
            current_std: 2,
            range: 40..60
        }
    };
    let optimal = find_optimal(&mut ind_var).await?;
    println!("{:?}", optimal); */
    
/*     // Run the quick simulation
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_participants: 15,
        average_proximity: 0.5,
        witness_floor: 2,
        runs: 100,
        reliability: vec![1.0, 1.0, 0.4, 0.7, 0.6, 0.8, 0.9, 0.7, 0.3, 0.6, 1.0, 0.7, 0.4, 0.5, 1.0],
        reliability_threshold: vec![0.1; 15],
        default_reliability: vec![0.5; 15],
        organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2]
    };
    let dir_name = witness_rep::quick_simulation::quick_simulation(sc).await?;

    // evaluate the results
    let rel_map = evaluating_rep::stats::read_reliabilities(dir_name, false)?;
    let mse = evaluating_rep::stats::run_avg_mean_squared_error(rel_map)?;
    println!("{}", mse); */

    // Run the quick simulation
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_participants: 15,
        average_proximity: 0.3,
        witness_floor: 2,
        runs: 4,
        reliability: vec![1.0, 1.0, 0.4, 0.7, 0.6, 0.8, 0.9, 0.7, 0.3, 0.6, 1.0, 0.7, 0.4, 0.5, 1.0],
        reliability_threshold: vec![0.1; 15],
        default_reliability: vec![0.5; 15],
        organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2]
    };
    let dir_name = witness_rep::simulation::simulation(sc).await?;

    // evaluate the results
    let rel_map = evaluating_rep::stats::read_reliabilities(dir_name, false)?;
    let mse = evaluating_rep::stats::run_avg_mean_squared_error(rel_map)?;
    println!("{}", mse); 

/*     let ann = "10a54dd01a48799c8def58b315a85b2aa62ccd3ca443c75350234054d11230160000000000000000:d5227ec97cc8597a471b9478";
    let seed = "Participant 0";
    let channel_msgs = witness_rep::utility::read_msgs::read_msgs(url, ann, seed).await?;
    let branch_msgs = witness_rep::utility::extract_msgs::extract_msg(channel_msgs, witness_rep::utility::verify_tx::WhichBranch::LastBranch);
    println!("{:?}", branch_msgs[0]); */

    Ok(())
}

pub fn run_eval_with_arg() -> Result<()>{
    let args: Vec<String> = env::args().collect();
    let rel_map = evaluating_rep::stats::read_reliabilities(args[1].clone(), true)?;
    let mse = evaluating_rep::stats::run_avg_mean_squared_error(rel_map)?;
    println!("{}", mse);
    return Ok(());
}