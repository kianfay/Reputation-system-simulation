use crate::{
    witness_rep,
    evaluating_rep::optimise::*
};

use anyhow::Result;

pub async fn evaluate_witness_floor_var(url: &str) -> Result<()> {
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_users: 15,
        average_proximity: 0.6,
        witness_floor: 2,
        runs: 30,
        reliability: vec![0.8; 15],
        user_reputation_threshold: vec![0.1; 15],
        user_default_reputation: vec![0.5; 15],
        user_organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2],
        organization_reputation_threshold: vec![0.1; 15],
        organization_default_reputation: vec![0.5; 15]
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