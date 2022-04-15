use crate::{
    witness_rep,
    evaluating_rep::optimise::*,
    evaluating_rep
};

use anyhow::Result;

pub async fn run_moderate_sim(url: &str) -> Result<()> {
    // Run the quick simulation
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_users: 15,
        average_proximity: 0.5,
        witness_floor: 2,
        runs: 100,
        reliability: vec![1.0, 1.0, 0.4, 0.7, 0.6, 0.8, 0.9, 0.7, 0.3, 0.6, 1.0, 0.7, 0.4, 0.5, 1.0],
        user_reliability_threshold: vec![0.1; 15],
        user_default_reliability: vec![0.5; 15],
        user_organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2],
        organization_reliability_threshold: vec![0.1; 15],
        organization_default_reliability: vec![0.5; 15]
    };
    let dir_name = witness_rep::simulation::simulation(sc).await?;

    // evaluate the results
    let rel_map = evaluating_rep::stats::read_reliabilities(dir_name, false)?;
    let mse = evaluating_rep::stats::run_avg_mean_squared_error(rel_map)?;
    println!("{}", mse);
    return Ok(());
}