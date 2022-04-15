use anyhow::Result;
use std::env;
use evaluating_rep::optimise::*;

mod examples;
mod witness_rep;
mod evaluating_rep;

#[tokio::main]
async fn main() -> Result<()> {
    let url = "http://0.0.0.0:14265";

    //evaluate_user_reliability_threshold_var(url).await?;
    examples::run_simple_sim::run_simple_sim(url).await?;


/*     let ann = "10a54dd01a48799c8def58b315a85b2aa62ccd3ca443c75350234054d11230160000000000000000:d5227ec97cc8597a471b9478";
    let seed = "Participant 0";
    let channel_msgs = witness_rep::utility::read_msgs::read_msgs(url, ann, seed).await?;
    let branch_msgs = witness_rep::utility::extract_msgs::extract_msg(channel_msgs, witness_rep::utility::verify_interaction::WhichBranch::LastBranch);
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