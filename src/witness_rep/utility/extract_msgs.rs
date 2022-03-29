use crate::witness_rep::utility::verify_tx::WhichBranch;

use iota_streams::app_channels::api::tangle::{
    MessageContent, UnwrappedMessage
};
use identity::{
    did::MethodData
};

use std::collections::HashMap;


// Ectracts all message payloads and pubkeys from each branch. start_branch
// starts at 0.
pub fn extract_msg(
    retrieved_msgs: Vec<UnwrappedMessage>,
    branches: WhichBranch
) -> Vec<(String, String)> {
    /* retrieved_msgs
        .iter()
        .for_each(|msg| {
            let content = &msg.body;
            match content {
                MessageContent::SignedPacket {
                    pk,
                    public_payload,
                    masked_payload: _,
                } => {
                    let pay = String::from_utf8(public_payload.0.to_vec()).unwrap();
                    let pubk = MethodData::new_multibase(pk);
                    if let MethodData::PublicKeyMultibase(mbpub) = pubk {
                        messages.last_mut().unwrap().push((pay, mbpub));
                    } else {
                        panic!("Failed to decode public key")
                    }
                },
                MessageContent::Keyload  => {
                    // create a new vector for each keyload
                    let new_vec: Vec<(String, String)> = Vec::from([]);
                    messages.push(new_vec);
                }
                _ => {
                    panic!("Simulation only built to handle signed and keyload messages");
                }
            }
        }); */
    let mut last_branch = 0;
    let mut messages: HashMap<usize, Vec<(String, String)>> = HashMap::new();
    let new_vec: Vec<(String, String)> = Vec::from([]);
    
    retrieved_msgs
        .iter()
        .for_each(|msg| {
            let content = &msg.body;
            match content {
                MessageContent::SignedPacket {
                    pk,
                    public_payload,
                    masked_payload: _,
                } => {
                    // decode the message and check if it's a tx_msg
                    let mut pay = String::from_utf8(public_payload.0.to_vec()).unwrap();
                    let run = &pay[0..1].to_string();
                    let run_i = run.parse::<usize>().unwrap();

                    if run_i > last_branch {
                        last_branch = run_i;
                    }

                    pay = (&pay[1..]).to_string();

                    let substr = &pay[0..16];
                    if substr == "{\"InteractionMsg" {
                        messages.insert(run_i, new_vec.clone());
                    }

                    let pubk = MethodData::new_multibase(pk);
                    if let MethodData::PublicKeyMultibase(mbpub) = pubk {
                        let mut branch = messages.remove(&run_i).unwrap();
                        branch.push((pay, mbpub));
                        messages.insert(run_i, branch);
                    } else {
                        panic!("Failed to decode public key")
                    }
                },
                _ => {}
            }
        });
    
    // TODO OneBranch and FromBranch
    return match branches {
        WhichBranch::OneBranch(b)  => messages.remove(&last_branch).unwrap(),
        WhichBranch::FromBranch(b) => messages.remove(&last_branch).unwrap(),
        WhichBranch::LastBranch  => messages.remove(&last_branch).unwrap(),
    };
}