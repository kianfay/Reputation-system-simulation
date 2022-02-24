use crate::witness_rep::utility::verify_tx::WhichBranch;

use iota_streams::app_channels::api::tangle::{
    MessageContent, UnwrappedMessage
};
use identity::{
    did::MethodData
};


// Ectracts all message payloads and pubkeys from each branch. start_branch
// starts at 0.
pub fn extract_msg(
    retrieved_msgs: Vec<UnwrappedMessage>,
    branches: WhichBranch
) -> Vec<Vec<(String, String)>> {
    let mut messages: Vec<Vec<(String, String)>> = Vec::new();

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
                    let pay = String::from_utf8(public_payload.0.to_vec()).unwrap();
                    println!("Next Debug: {}\n", pay);

                    // first 16 chars = {"TransactionMsg
                    let substr = &pay[0..16];
                    if substr == "{\"TransactionMsg" {
                        
                        messages.push(new_vec.clone());
                    }

                    let pubk = MethodData::new_multibase(pk);
                    if let MethodData::PublicKeyMultibase(mbpub) = pubk {
                        messages.last_mut().unwrap().push((pay, mbpub));
                    } else {
                        panic!("Failed to decode public key")
                    }
                },
                _ => {}
            }
        });
    
    
    return match branches {
        WhichBranch::OneBranch(b)  => vec![Vec::from(messages[b].clone())],
        WhichBranch::FromBranch(b) => messages.into_iter()
                                        .enumerate()
                                        .filter(|(i,_)| *i >= b)
                                        .map(|(_,br)| br)
                                        .collect(),
        WhichBranch::LastBranch  => vec![Vec::from(messages[messages.len()-1].clone())],
    };
}