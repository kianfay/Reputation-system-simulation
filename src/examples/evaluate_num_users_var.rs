use crate::{
    witness_rep,
    evaluating_rep::optimise::*
};

use anyhow::Result;

pub async fn evaluate_num_users_var(url: &str) -> Result<()> {
    let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_users: 4,
        average_proximity: 0.5,
        witness_floor: 2,
        runs: 10,
        reliability: vec![0.8; 4],
        user_reliability_threshold: vec![0.1; 4],
        user_default_reliability: vec![0.5; 4],
        user_organizations: vec![0,1,2,3],
        organization_reliability_threshold: vec![0.1; 4],
        organization_default_reliability: vec![0.5; 4]
    };

    let mut ind_var_0: IndependantVar<IndependantVarApp> = IndependantVar {
        sc: sc.clone(),
        independant_var: IndependantVarApp {
            independant_var: IndependantVarAppHollow::NumParticipants(4..16)
        }
    };

    let optimal = find_optimal(&mut ind_var_0).await?;
    println!("{:?}", optimal);

    return Ok(());
}