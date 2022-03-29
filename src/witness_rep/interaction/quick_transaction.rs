use crate::witness_rep::{
    interaction::{
        generate_sigs, 
        participant::{
            ParticipantIdentity, OrganizationIdentity, IdInfo, get_public_keys
        },
        interaction::{extract_from_ids, get_honest_nodes, lazy_outcome, LazyMethod}
    },
};

use trust_score_generator::trust_score_generators::{
    data_types::{
        messages::{
            tx_messages as message,
            contract::{Contract, PublicKey},
            signatures::{
                witness_sig, interaction_sig, organization_cert, 
            },
            tx_messages::{
                WitnessClients, ArrayOfWnSignitures, ArrayOfTxSignitures
            }
        },
        message as tsg_message
    }
};

use iota_streams::{
    app::transport::tangle::{TangleAddress, client::Client},
    app_channels::api::tangle::{
        Address, Author, Bytes, Subscriber
    },
    core::{println, Result},
    app::message::HasLink
};
use identity::{
    did::MethodData,
    crypto::KeyPair
};
use rand::Rng;
use core::str::FromStr;

pub async fn quick_transact(
    contract: Contract,
    transacting_ids: &mut Vec<ParticipantIdentity>,
    witness_ids: &mut Vec<ParticipantIdentity>,
    organization_id: &mut OrganizationIdentity,
    lazy_method: LazyMethod,
    run: usize,
    print: bool
) -> Result<Option<(Vec<bool>, Vec<bool>, Vec<tsg_message::MessageAndPubkey>)>> {
    const DEFAULT_TIMEOUT : u32 = 60*2; // 2 mins
    let mut messages: Vec<tsg_message::MessageAndPubkey> = Vec::new();

    //--------------------------------------------------------------
    //--------------------------------------------------------------
    // EXTRACT CLIENTS AND KEYPAIRS FROM IDENTITIES
    //--------------------------------------------------------------
    let (mut transacting_clients, transacting_did_kp, transacting_reliablity, transacting_org_certs) = extract_from_ids(transacting_ids)?;
    let (mut witness_clients, witness_did_kp, witness_reliability, witness_org_certs) = extract_from_ids(witness_ids)?;

    //--------------------------------------------------------------
    // ORGANIZATION CHECKS THE RELIABILITIES OF THE PARTICIPANTS
    //--------------------------------------------------------------
    
    // get the public keys of all the participants
    let mut tn_pks = get_public_keys(&transacting_org_certs);
    let mut wn_pks = get_public_keys(&witness_org_certs);
    let mut participant_pks = Vec::new();
    participant_pks.append(&mut tn_pks); participant_pks.append(&mut wn_pks);

    if !organization_id.identity.check_avg_participants(&participant_pks){
        //panic!("The average reliability of the participants does not satisfy the organizations threshold")
        return Ok(None);
    }


    //--------------------------------------------------------------
    // ORGANIZATION SENDS KEYLOAD
    //--------------------------------------------------------------



    //--------------------------------------------------------------
    // WITNESSES GENERATE SIGS
    //--------------------------------------------------------------
    if print {
        println!("Witnesses generate their signatures:");
    }
    let mut witness_sigs: Vec<witness_sig::WitnessSig> = Vec::new();
    let mut witness_sigs_bytes: Vec<Vec<u8>> = Vec::new();

    for i in 0..witness_clients.len() {
        let multibase_pub = MethodData::new_multibase(witness_clients[i].get_public_key());
        let channel_pk_as_multibase: String;
        if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
            channel_pk_as_multibase = mbpub;
        }
        else {
            panic!("Could not encode public key as multibase")
        }

        let sig = generate_sigs::generate_witness_sig(
            contract.clone(),
            channel_pk_as_multibase,
            witness_did_kp[i].clone(),
            witness_org_certs[i].clone(),
            DEFAULT_TIMEOUT
        )?;
        witness_sigs.push(sig.clone());

        // gets the signature of the hased WitnessSignature struct
        let sig_bytes = sig.signature;
        witness_sigs_bytes.push(sig_bytes);
    }
    if print {
        println!("-- Witness signatures generated\n");
    }

    //--------------------------------------------------------------
    // TRANSACTING NODES GENERATE SIGS
    //--------------------------------------------------------------

    let witnesses: Vec<PublicKey> = witness_did_kp
        .iter()
        .map(|kp| {
            let multibase_pub = MethodData::new_multibase(kp.public());
            if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
                return mbpub
            }
            else {
                panic!("Could not encode public key as multibase")
            }
        })
        .collect();

    if print {
        println!("Transacting nodes generate their signatures:");
    }
    let mut transacting_sigs: Vec<interaction_sig::InteractionSig> = Vec::new();
    for i in 0..transacting_clients.len() {
        let multibase_pub = MethodData::new_multibase(transacting_clients[i].get_public_key());
        let channel_pk_as_multibase: String;
        if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
            channel_pk_as_multibase = mbpub;
        }
        else {
            panic!("Could not encode public key as multibase")
        }
        let sig = generate_sigs::generate_transacting_sig(
            contract.clone(),
            channel_pk_as_multibase,
            transacting_did_kp[i].clone(),
            WitnessClients(witnesses.clone()),
            interaction_sig::ArrayOfWnSignituresBytes(witness_sigs_bytes.clone()),
            transacting_org_certs[i].clone(),
            DEFAULT_TIMEOUT
        )?;
        transacting_sigs.push(sig);
    }
    if print {
        println!("-- Transacting node signatures generated\n");
    }

    //--------------------------------------------------------------
    // INITIATING TN, HAVING REVEIVED THE SIGNATURES, 
    // BUILD FINAL INTERACTION (TN = TRANSACTING NODE)
    //--------------------------------------------------------------

    if print {
        println!("Initiating transacting node generates InteractionMessage:");
    }
    let interaction_msg = message::Message::InteractionMsg {
        contract: contract.clone(),
        witnesses: WitnessClients(witnesses.clone()),
        wit_node_sigs: ArrayOfWnSignitures(witness_sigs.clone()),
        tx_client_sigs: ArrayOfTxSignitures(transacting_sigs.clone()),
    };
    if print {
        println!("-- InteractionMessage generated");
    }
    
    //--------------------------------------------------------------
    // INITIATING TN SENDS THE TRANSACTION MESSAGE
    //--------------------------------------------------------------

    let msg = tsg_message::MessageAndPubkey {
        message: interaction_msg,
        sender_did: transacting_org_certs[0].client_pubkey.clone()
    };
    messages.push(msg);

    //--------------------------------------------------------------
    // THE EVENT IN QUESTION ON THE CONTRACT PLAYS OUT
    // (WE GENERATE THE OUTCOME AS PART OF THE SIMULATION)
    //--------------------------------------------------------------


    // Dishonest transacting nodes still want to get compensated, but are rellying
    // on lazy (or colluding) witnesses for compensation to be more likely. Reason
    // being, the counterparty may still compensate them even if they act dishonestly,
    // but only if the witnesses side with the dishonest node, thus jepordising the 
    // the conterparties trust score.
    if print {
        println!("Assigning tranascting nodes as (dis)honest according to their reliability:");
    }
    let honest_tranascting_ids = get_honest_nodes(transacting_reliablity, 1);
    if print {
        println!("Assigning witnesses as (dis)honest according to their reliability:");
    }
    let honest_witness_ids = get_honest_nodes(witness_reliability, 0);

    // A vector of vectors, the inner a list of the outcomes per participant from
    // the witnesses point of view.
    if print {
        println!("Witnesses decide on the outcome:");
    }
    let mut outcomes: Vec<Vec<bool>> = vec![Vec::new(); honest_witness_ids.len()];
    for i in 0..honest_witness_ids.len() {
        let honesty_of_wn = honest_witness_ids[i];

        // witness determines the outcome for each participant
        for j in 0..honest_tranascting_ids.len() {
            let honesty_of_tn = honest_tranascting_ids[j];
            
            // if the witness node is honest, then the output is dependant on whether
            // the tn was honest. Otherwise, it is either random or a constant. They may
            // want it to random so the trust score generator has a harder time seeing
            // their dishonesty.
            if honesty_of_wn {
                outcomes[i].push(honesty_of_tn);
                println!("-- Witnesses {} responds honestly about transacting node {}", i, j);
                if print {
        
                }
            } else {
                outcomes[i].push(lazy_outcome(&lazy_method));
                println!("-- Witnesses {} responds dishonestly about transacting node {}", i, j);
                if print {
        
                }
            }
        }
    }
    if print {
        println!("");
    }

    //--------------------------------------------------------------
    // WITNESSES SEND THEIR STATMENTS
    //--------------------------------------------------------------

    if print {
        println!("Witnesses generate and send their witness statements:");
    }
    for i in 0..witness_clients.len(){
        // WN's prepares their statement
        let wn_statement = message::Message::WitnessStatement {
            outcome: outcomes[i].clone()
        };

        let msg = tsg_message::MessageAndPubkey {
            message: wn_statement,
            sender_did: witness_ids[i].id_info.org_cert.client_pubkey.clone()
        };
        messages.push(msg);
    }
    if print {
        println!("");
    }

    //--------------------------------------------------------------
    // THE PARTICIPANTS READ THE STATEMENTS AND DECIDE TO COMPENSATE
    // OR NOT (NOT WOULD IN PRINCIPAL BE A DISHONEST CHOICE)
    //--------------------------------------------------------------

    // TODO - add read and choice

    if print {
        println!("Transacting nodes send compensation:");
    }
    for i in 0..transacting_clients.len(){

        // TODO - certain TNs need to compensate other TNs

        // TN prepares the compensation interaction 
        let payments_tn_a = vec![
            //"tn_b: 0.1".to_string(),
            "wn_a: 0.01".to_string(),
            "wn_b: 0.01".to_string()
        ];
        let compensation_msg = message::Message::CompensationMsg {
            payments: payments_tn_a
        };

        let msg = tsg_message::MessageAndPubkey {
            message: compensation_msg,
            sender_did: transacting_ids[i].id_info.org_cert.client_pubkey.clone()
        };
        messages.push(msg);
    }
    if print {
        println!("");
    }
    
    return Ok(Some((honest_tranascting_ids, honest_witness_ids, messages)));
}