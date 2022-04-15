use crate::{
    witness_rep,
    evaluating_rep::optimise::*,
    evaluating_rep
};

use anyhow::Result;

pub async fn run_simple_sim(url: &str) -> Result<()> {
    // Run the quick simulation
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_users: 4,
        average_proximity: 1.0,
        witness_floor: 2,
        runs: 10,
        reliability: vec![1.0, 1.0, 0.4, 0.7],
        user_reliability_threshold: vec![0.1; 4],
        user_default_reliability: vec![0.5; 4],
        user_organizations: vec![0,1,1,2],
        organization_reliability_threshold: vec![0.1; 4],
        organization_default_reliability: vec![0.5; 4]
    };
    let dir_name = witness_rep::simulation::simulation(sc).await?;

    // evaluate the results
    let rel_map = evaluating_rep::stats::read_reliabilities(dir_name, false)?;
    let mse = evaluating_rep::stats::run_avg_mean_squared_error(rel_map)?;
    println!("{}", mse);
    return Ok(());
}