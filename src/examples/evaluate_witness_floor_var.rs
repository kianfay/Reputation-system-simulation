use crate::{
    witness_rep,
    evaluating_rep::optimise::*
};

use anyhow::Result;

pub async fn evaluate_witness_floor_var(url: &str) -> Result<()> {
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_participants: 15,
        average_proximity: 0.6,
        witness_floor: 2,
        runs: 10,
        reliability: vec![0.8; 15],
        reliability_threshold: vec![0.1; 15],
        default_reliability: vec![0.5; 15],
        organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2]
    };

    let mut ind_var_0: IndependantVar<IndependantVarApp> = IndependantVar {
        sc: sc.clone(),
        independant_var: IndependantVarApp {
            independant_var: IndependantVarAppHollow::WitnessFloor(2..8)
        }
    };

    let optimal = find_optimal(&mut ind_var_0).await?;
    println!("{:?}", optimal);

    return Ok(());
}