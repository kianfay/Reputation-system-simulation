use anyhow::Result;

mod witness_rep;

#[tokio::main]
async fn main() -> Result<()> {
    let url = "http://0.0.0.0:14265";

/*     let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_participants: 15,
        average_proximity: 0.7,
        witness_floor: 2,
        runs: 10,
        reliability: vec![0.75, 0.5, 0.4, 0.7, 0.6, 0.8, 0.9, 0.7, 0.3, 0.6, 0.5, 0.7, 0.4, 0.5, 0.8],
        reliability_threshold: vec![0.1; 15],
        default_reliability: vec![0.5; 15],
        organizations: vec![0,0,1,1,2,2,1,1,0,1,0,2,2,1,0]
    };

    witness_rep::simulation::simulation(sc).await?; */

/*     let sc = witness_rep::simulation::SimulationConfig {
        node_url: String::from(url),
        num_participants: 4,
        average_proximity: 1.0,
        witness_floor: 2,
        runs: 2,
        reliability: vec![1.0; 4],
        reliability_threshold: vec![0.5; 4],
        default_reliability: vec![0.5; 4],
        organizations: vec![1,1,1,1,1]
    };

    witness_rep::simulation::simulation(sc).await?; */

    let ann = "9f03c278ec29e639ae621cc3cc054c75b345863f5119088a2985b367dae1c2600000000000000000:768f2c1e0cf4fcc27dffbe14";
    let seed = "OXQZDVLBSRKJUZHQNSQUGKSXAJEZVP9IHQQOJPBUQZVPLPTMPJBVIOVZTDRUK9RJQCWNCDUMREDUBPFIG";
    let channel_msgs = witness_rep::utility::read_msgs::read_msgs(url, ann, seed).await?;
    let branch_msgs = witness_rep::utility::extract_msgs::extract_msg(channel_msgs, witness_rep::utility::verify_tx::WhichBranch::LastBranch);
    println!("{:?}", branch_msgs[0]);

    Ok(())
}
