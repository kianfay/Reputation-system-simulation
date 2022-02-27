use crate::witness_rep::{
    utility::{extract_msgs}
};

use trust_score_generator::trust_score_generators::{
    data_types::{
        messages::{
            tx_messages as message,
            signatures::{
                witness_sig, transacting_sig
            },
        }
    },
};

use iota_streams::{
    app_channels::api::tangle::UnwrappedMessage,
    core::{println, Result},
};
use identity::{
    crypto::{Ed25519, Verify},
    did::MethodData,
};

use ed25519_dalek::{Sha512, Digest};

#[derive(Clone,PartialEq,Debug)]
pub enum PublickeyOwner {
    TransactingNode(String),
    Witness(String)
}

pub fn extract_wn_pks(pks: Vec<PublickeyOwner>) -> message::WitnessClients {
    let pks = pks.iter().filter(|p| {
        match p {
            PublickeyOwner::Witness(_)  => true,
            _                           => false
        }
    }).map(|w| {
        match w {
            PublickeyOwner::Witness(s)  => *s,
            _                           => String::default()
        }
    })
    .collect::<Vec<String>>();

    return message::WitnessClients(pks);
}

pub enum WhichBranch {
    /// Verifies a specific branch
    OneBranch(usize),
    /// Verifies all branches including and after a specific branch
    FromBranch(usize),
    /// Verifies only the final branch
    LastBranch
}

/// Returns whether the transaction msgs were valid, the messages, and the channel pks which signed the msgs
pub async fn verify_txs(
    msgs: Vec<UnwrappedMessage>,
    branches: WhichBranch
) -> Result<(bool, Vec<String>, Vec<String>)> {
    
    let msgs = extract_msgs::extract_msg(msgs, branches);
/*     let msgs: Vec<(String, String)> = branches_msgs.into_iter()
        .flatten()
        .collect(); */

    let only_msgs = msgs.iter().map(|(msg, _)| msg.clone()).collect();
    let only_pks = msgs.iter().map(|(_, pk)| pk.clone()).collect();

    // parse the string into the TransactionMsg/WitnessStatement/CompensationMsg format and check if valid
    let mut valid_pks: Vec<PublickeyOwner> = Vec::new();
    for (cur_msg, pk) in msgs.iter() {
        let deserialised_msg: message::Message = serde_json::from_str(cur_msg.as_str())?;
        let verified = verify_msg((deserialised_msg,pk), valid_pks.clone())?;

        let final_verify = match verified {
            (true, Some(ret_pk))=> {
                let mut rte_pk_clone = ret_pk.clone();
                valid_pks.append(&mut rte_pk_clone);
                true
            },
            (true, None)        => true,
            (false, _)          => false
        };
        //println!("Valid pks: {:?}", valid_pks);

        println!("Verified status of msg: {}", final_verify);
        if !final_verify {
            return Ok((false, only_msgs, only_pks));
        }
    }

    return Ok((true, only_msgs, only_pks));
}

/// Accepts a tuple of a message content and the sender's channel public key.
/// If it is a valid TransactionMessage, it will return true and a valid channel public keys and it's ownership
pub fn verify_msg( (tx_msg,channel_pk) : (message::Message, &String), mut valid_pks: Vec<PublickeyOwner>) -> Result<(bool, Option<Vec<PublickeyOwner>>)> {
    //println!("EEERRREEEHH: {:?}", tx_msg);
    match tx_msg {
        message::Message::TransactionMsg {
            contract, witnesses, wit_node_sigs, tx_client_sigs
        } => {
            let tx_msg = message::Message::TransactionMsg {contract, witnesses, wit_node_sigs, tx_client_sigs};
            let (message::ArrayOfWnSignitures(wit_sigs), message::ArrayOfTxSignitures(tn_sigs)) = get_sigs(tx_msg);
            
            // Check that each witness sig is valid, meaning it was sent by the owner of the DID,
            // not just any person who holds the public key of the DID
            for ws in wit_sigs.iter() {
                let (verified, pk, sig) = verify_witness_sig(ws.clone())?;
                if !verified {
                    panic!("Signature verification failed")
                } else {
                    valid_pks.push(PublickeyOwner::Witness(pk));
                }
            }

            // Check that each tn sig is valid, similarly to the witness sig, but additionally
            // that each participant has agreed to the witnesses specified in the tx_msg. This
            // can be done by simply checking that all they have all the witness sigs in their
            // own sig. By checking the sig not the pubkey of the witness, we ensure that the tn
            // is agreeing to a specific instance of the witness agreeing to witness, not simply
            // agreeing to have a witness witness the event.
            witness_sigs.sort();
            for ts in tn_sigs.iter() {
                let (verified, pk) = verify_tx_sig(ts.clone(), witness_sigs.clone())?;
                if !verified {
                    panic!("Signature verification failed")
                } else {
                    valid_pks.push(PublickeyOwner::TransactingNode(pk));
                }
            }
            return Ok((true, Some(valid_pks)))
        },
        message::Message::WitnessStatement {
            outcome: _,
        } => {
            //println!("Inside here");
            let wrapped_channel_pk = PublickeyOwner::Witness(channel_pk.clone());
            //println!("{:?}", wrapped_channel_pk);
            if valid_pks.contains(&wrapped_channel_pk) {
                return Ok((true, None));
            }
        },
        message::Message::CompensationMsg {
            payments: _
        } => {
            //println!("Inside here");
            let wrapped_channel_pk = PublickeyOwner::TransactingNode(channel_pk.clone());
            //println!("CHANNEL_PK: {:?}", wrapped_channel_pk);
            if valid_pks.contains(&wrapped_channel_pk) {
                return Ok((true, None));
            }
        }
    }
    //println!("Unfort here");
    return Ok((false, None));
}

pub fn get_sigs(
    tx: message::Message
) -> (message::ArrayOfWnSignitures,message::ArrayOfTxSignitures) {
    match tx {
        message::Message::TransactionMsg {
            contract: _,
            witnesses: _,
            wit_node_sigs,
            tx_client_sigs,
        } => return (wit_node_sigs, tx_client_sigs),
        _ => panic!("Can only get signatures from a TransactionMsg message")
    };
}

/// Returns a bool indicating if valid, and a string of the channel pubkey of this sub,
/// and the sig bytes
pub fn verify_witness_sig(
    sig: witness_sig::WitnessSig
) -> Result<(bool, String, Vec<u8>)>{
    match sig {
        witness_sig::WitnessSig {
            contract,
            signer_channel_pubkey,
            org_cert,
            timeout,
            signer_did_pubkey,
            signature,
        } => {
            let pre_sig = witness_sig::WitnessPreSig {
                contract,
                signer_channel_pubkey: signer_channel_pubkey.clone(),
                org_cert: org_cert.clone(),
                timeout,
            };
            //println!("IMPORTANT: {:?}", pre_sig);

            let pre_sig = serde_json::to_string(&pre_sig).unwrap();

            let signer_did_pubkey = MethodData::PublicKeyMultibase(signer_did_pubkey);
            let decoded_pubkey = MethodData::try_decode(&signer_did_pubkey)?;
            let sig_unsigned = Ed25519::verify(pre_sig.as_bytes(), &signature, &decoded_pubkey);
            if let Ok(()) = sig_unsigned {
                return Ok((true,signer_channel_pubkey,signature));
            } else {
                panic!("Signature verification failed")
            }
        }
    }
}

// returns a bool indicating if valid, and a string of the channel pubkey of this sub
pub fn verify_tx_sig(
    sig: transacting_sig::TransactingSig,
    witnesses: message::WitnessClients,
    hash_len: usize
) -> Result<(bool, String)>{
    match sig {
        transacting_sig::TransactingSig {
            contract,
            signer_channel_pubkey,
            wit_node_sigs,
            org_cert,
            timeout,
            signer_did_pubkey,
            signature,
        } => {

            // generate the hash
            witnesses.sort();
            let ser_wns = serde_json::to_string(&witnesses)?;
            let hash_wns = Sha512::digest(ser_wns.as_bytes());
            let trun_hash_wns = Vec::from(&hash_wns[0..hash_len]);

            let pre_sig = transacting_sig::TransactingPreSig {
                contract,
                signer_channel_pubkey: signer_channel_pubkey.clone(),
                wit_node_sigs,
                org_cert: org_cert.clone(),
                timeout,
            };

            let pre_sig = serde_json::to_string(&pre_sig).unwrap();

            let signer_did_pubkey = MethodData::PublicKeyMultibase(signer_did_pubkey);
            let decoded_pubkey = MethodData::try_decode(&signer_did_pubkey)?;
            let sig_unsigned = Ed25519::verify(pre_sig.as_bytes(), &signature, &decoded_pubkey);
            if let Ok(()) = sig_unsigned {
                return Ok((true,signer_channel_pubkey));
            } else {
                panic!("Signature verification failed")
            }
        }
    }
}

/* pub fn testing() -> Result<()> {
    let tx = message::Message::WitnessStatement {
        outcome: true
    };
    let txstr = serde_json::to_string(&tx)?;
    println!("{}", txstr);
    println!("{{\"outcome\": \"true\"}}");
    let txback : message::Message = serde_json::from_str("{\"outcome\": \"true\"}")?;
    //let txback : message::Message = serde_json::from_str("{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"witnesses\":[\"z88JJehsqhqYeR8mPLn76grqy35Dn7q4iZNxbchXLMCyy\",\"zJB3y431YDHLUohS4uT6bjDikttuP8wsxcVmv2VzhEYxr\"],\"wit_node_sigs\":[{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"z84iDAGmmt4uK8PHbyCKWzfmApagUxyttVQSzi8gRAr8o\",\"timeout\":120,\"signer_did_pubkey\":\"z88JJehsqhqYeR8mPLn76grqy35Dn7q4iZNxbchXLMCyy\",\"signature\":[162,63,75,203,166,227,226,172,10,90,33,231,7,221,151,94,92,91,32,195,191,92,186,34,230,158,211,61,185,229,49,58,144,2,103,19,93,255,57,166,71,92,17,8,158,158,11,12,247,22,45,70,128,21,138,218,78,185,68,160,194,142,196,4]}},{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"zGPzWmjMbFmXmjFNWcjwxXEiYxiYYmabHt7n1TCP8fnEF\",\"timeout\":120,\"signer_did_pubkey\":\"zJB3y431YDHLUohS4uT6bjDikttuP8wsxcVmv2VzhEYxr\",\"signature\":[21,3,215,246,167,52,117,29,144,62,100,37,195,205,25,43,163,41,51,30,233,251,38,153,244,106,211,200,4,61,210,67,75,10,197,224,212,84,192,227,27,32,0,29,53,171,187,250,126,173,80,86,245,64,204,239,165,45,109,207,105,79,233,10]}}],\"tx_client_sigs\":[{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"zBmKHy6koZQumcYA9nQbfi82Bzrqk36UYbdMKp2qqxGxe\",\"witnesses\":[\"z88JJehsqhqYeR8mPLn76grqy35Dn7q4iZNxbchXLMCyy\",\"zJB3y431YDHLUohS4uT6bjDikttuP8wsxcVmv2VzhEYxr\"],\"wit_node_sigs\":[{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"z84iDAGmmt4uK8PHbyCKWzfmApagUxyttVQSzi8gRAr8o\",\"timeout\":120,\"signer_did_pubkey\":\"z88JJehsqhqYeR8mPLn76grqy35Dn7q4iZNxbchXLMCyy\",\"signature\":[162,63,75,203,166,227,226,172,10,90,33,231,7,221,151,94,92,91,32,195,191,92,186,34,230,158,211,61,185,229,49,58,144,2,103,19,93,255,57,166,71,92,17,8,158,158,11,12,247,22,45,70,128,21,138,218,78,185,68,160,194,142,196,4]}},{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"zGPzWmjMbFmXmjFNWcjwxXEiYxiYYmabHt7n1TCP8fnEF\",\"timeout\":120,\"signer_did_pubkey\":\"zJB3y431YDHLUohS4uT6bjDikttuP8wsxcVmv2VzhEYxr\",\"signature\":[21,3,215,246,167,52,117,29,144,62,100,37,195,205,25,43,163,41,51,30,233,251,38,153,244,106,211,200,4,61,210,67,75,10,197,224,212,84,192,227,27,32,0,29,53,171,187,250,126,173,80,86,245,64,204,239,165,45,109,207,105,79,233,10]}}],\"timeout\":120,\"signer_did_pubkey\":\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"signature\":[183,205,116,252,46,63,163,254,194,207,62,1,137,134,209,178,135,142,176,115,232,254,135,241,129,141,245,210,186,91,240,35,198,81,220,31,53,121,174,199,51,248,138,58,49,50,170,139,22,204,136,171,1,34,229,54,25,13,174,57,159,174,43,7]}},{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"zHHRYyM584S8bnoWeG8vLTzy24EPif3rNPgQ862Qnqy1m\",\"witnesses\":[\"z88JJehsqhqYeR8mPLn76grqy35Dn7q4iZNxbchXLMCyy\",\"zJB3y431YDHLUohS4uT6bjDikttuP8wsxcVmv2VzhEYxr\"],\"wit_node_sigs\":[{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"z84iDAGmmt4uK8PHbyCKWzfmApagUxyttVQSzi8gRAr8o\",\"timeout\":120,\"signer_did_pubkey\":\"z88JJehsqhqYeR8mPLn76grqy35Dn7q4iZNxbchXLMCyy\",\"signature\":[162,63,75,203,166,227,226,172,10,90,33,231,7,221,151,94,92,91,32,195,191,92,186,34,230,158,211,61,185,229,49,58,144,2,103,19,93,255,57,166,71,92,17,8,158,158,11,12,247,22,45,70,128,21,138,218,78,185,68,160,194,142,196,4]}},{{\"contract\":{{\"contract_definition\":\"tn_b allows tn_a to enter in front of it in the lane tn_b is in\",\"participants\":[\"zAf95pggBY7aDjZgDCwsp1bWJZaGQSrQ2w8Njtx9NTt9Z\",\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\"],\"time\":1643572739,\"location\":[[53,20,27.036],[6,15,2.695]]}},\"signer_channel_pubkey\":\"zGPzWmjMbFmXmjFNWcjwxXEiYxiYYmabHt7n1TCP8fnEF\",\"timeout\":120,\"signer_did_pubkey\":\"zJB3y431YDHLUohS4uT6bjDikttuP8wsxcVmv2VzhEYxr\",\"signature\":[21,3,215,246,167,52,117,29,144,62,100,37,195,205,25,43,163,41,51,30,233,251,38,153,244,106,211,200,4,61,210,67,75,10,197,224,212,84,192,227,27,32,0,29,53,171,187,250,126,173,80,86,245,64,204,239,165,45,109,207,105,79,233,10]}}],\"timeout\":120,\"signer_did_pubkey\":\"z69fT9PoLLjij8ZofrHyLpUkU62spCB6Waqjh7eF3jJE1\",\"signature\":[35,201,117,1,58,220,198,161,105,28,156,52,19,90,116,97,176,101,81,11,157,81,184,3,248,148,1,254,48,214,191,60,174,153,39,37,4,135,252,241,234,203,14,171,49,254,86,93,0,132,190,58,238,230,29,245,55,10,91,29,146,59,103,8]}}]}}")?;
    match txback {
        message::Message::WitnessStatement { outcome} => println!("Witness {}", outcome),
        message::Message::TransactionMsg { contract,witnesses,wit_node_sigs,tx_client_sigs} => println!("Tx"),
        _ => println!("neither"),
    }
    return Ok(());
} */