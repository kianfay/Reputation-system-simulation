use crate::{
    witness_rep,
    evaluating_rep::optimise::*
};

use anyhow::Result;

pub async fn evaluate_user_default_reliability_var(url: &str) -> Result<()> {
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_users: 15,
        average_proximity: 0.6,
        witness_floor: 2,
        runs: 10,
        reliability: vec![0.8; 15],
        user_reliability_threshold: vec![0.1; 15],
        user_default_reliability: vec![0.5; 15],
        user_organizations: vec![0,0,0,0,0,1,1,1,1,1,2,2,2,2,2],
        organization_reliability_threshold: vec![0.1; 15],
        organization_default_reliability: vec![0.5; 15]
    };

    let mut ind_var_0: IndependantVar<IndependantVarPart> = IndependantVar {
        sc: sc.clone(),
        independant_var: IndependantVarPart {
            independant_var: IndependantVarPartHollow::DefaultReliability,
            current_mean: 40,
            current_std: 2,
            range: 40..100
        }
    };
    let optimal = find_optimal(&mut ind_var_0).await?;
    println!("{:?}", optimal);

    return Ok(());
}