use crate::{
    witness_rep,
    evaluating_rep::optimise::*
};

use anyhow::Result;

pub async fn evaluate_reliability_var(url: &str) -> Result<()> {
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

    let mut ind_var_0: IndependantVar<IndependantVarPart> = IndependantVar {
        sc: sc.clone(),
        independant_var: IndependantVarPart {
            independant_var: IndependantVarPartHollow::Reliability,
            current_mean: 0,
            current_std: 10,
            range: 0..100
        }
    };
    let optimal = find_optimal(&mut ind_var_0).await?;
    println!("{:?}", optimal);

    return Ok(());
}