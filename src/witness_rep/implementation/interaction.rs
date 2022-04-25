use crate::witness_rep::{
    implementation::{
        generate_sigs, 
        user_and_organization::{
            UserIdentity, OrganizationIdentity, IdInfo, get_public_keys
        }
    },
};

use wb_reputation_system::data_types::{
    event_protocol_messages::{
        application_constructs::{
            application_messages::{
                exchange_app_messages::CompensationMsg,
            },
            application_contracts::{
                utility_types::{
                    PublicKey, WitnessUsers
                }
            },
        },
        event_protocol_messages::{
            Contract, Message,
            ArrayOfWnSignitures, ArrayOfIntSignitures,
            ApplicationMsg, Outcome
        },
        signatures::{
            witness_sig, interaction_sig,
            organization_cert::{
                OrganizationCertificate
            }, 
        },
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

#[derive(Clone, Debug)]
pub enum LazyMethod {
    Constant(bool),
    Random,
}

//pub type OrganizationIdentity = Identity<Author<Client>>;

pub fn extract_from_id(
    id: &mut UserIdentity
) -> Result<(&mut Subscriber<Client>, KeyPair, f32, OrganizationCertificate)> {
    match &id.id_info {
        IdInfo { 
            did_key,
            reliability,
            org_cert,
            seed: _
        } => {
            let did_keypair = KeyPair::try_from_ed25519_bytes(did_key)?;
            return Ok((&mut id.channel_client, did_keypair,reliability.clone().unwrap(), org_cert.clone()));
        }
    }
}

pub fn extract_from_ids(
    ids: &mut Vec<UserIdentity>
) -> Result<(Vec<&mut Subscriber<Client>>, Vec<KeyPair>, Vec<f32>, Vec<OrganizationCertificate>)> {
    let mut subs: Vec<&mut Subscriber<Client>>  = Vec::new();
    let mut kps : Vec<KeyPair>                  = Vec::new();
    let mut rels: Vec<f32>                      = Vec::new();
    let mut orgs: Vec<OrganizationCertificate>      = Vec::new();

    for id in ids {
        let (sub, kp, rel, org) = extract_from_id(id)?;
        subs.push(sub);
        kps.push(kp);
        rels.push(rel);
        orgs.push(org);
    }
    return Ok((subs, kps,rels,orgs));
}

pub async fn sync_all(subs: &mut Vec<&mut Subscriber<Client>>) -> Result<()> {
    for sub in subs {
        sub.sync_state().await?;
    }
    return Ok(());
}

/// Assigns users as being dishonest or honest, depending on their reliability
pub fn get_honest_users(users_reliablity: Vec<f32>) -> Vec<bool>{
    let mut honest_nodes: Vec<bool> = vec![true; users_reliablity.len()];

    // determine honesty/dishonesty for each user
    for i in 0..users_reliablity.len() {

        // randomly assert if they are acting honest based on their reliability
        let rand: f32 = rand::thread_rng().gen();
        println!("-- Trying participant {}. Rand={}", i, rand);
        let acting_honest: bool = users_reliablity[i] > rand;
        if !acting_honest {
            honest_nodes[i] = false;
            println!("---- Participant {} set to dishonest", i);
        } else {
            println!("---- Participant {} set to honest", i);
        }
    }
    println!("");

    return honest_nodes;
}

pub fn lazy_outcome(lazy_method: &LazyMethod) -> bool {
    return match lazy_method {
        LazyMethod::Constant(output) => output.clone(),
        LazyMethod::Random => {
            let rand: f32 = rand::thread_rng().gen();
            println!("-- Trying lazy outcome. Rand={}", rand);
            if rand > 0.5 {
                true
            } else {
                false
            }
        }
    }
}


pub async fn interaction(
    contract: Contract,
    participant_ids: &mut Vec<UserIdentity>,
    witness_ids: &mut Vec<UserIdentity>,
    organization_id: &mut OrganizationIdentity,
    lazy_method: LazyMethod,
    run: usize
) -> Result<Option<(Vec<bool>, Vec<bool>)>> {
    const DEFAULT_TIMEOUT : u32 = 60*2; // 2 mins
    let ann_str = organization_id.ann_msg.as_ref().unwrap();
    let announcement_link = Address::from_str(ann_str)?;

    //--------------------------------------------------------------
    //--------------------------------------------------------------
    // EXTRACT CLIENTS AND KEYPAIRS FROM IDENTITIES
    //--------------------------------------------------------------
    let (mut participant_clients, participant_did_kp, participant_reliablity, participant_org_certs) = extract_from_ids(participant_ids)?;
    let (mut witness_clients, witness_did_kp, witness_reliability, witness_org_certs) = extract_from_ids(witness_ids)?;

    //--------------------------------------------------------------
    // ORGANIZATION CHECKS THE RELIABILITIES OF THE PARTICIPANTS
    //--------------------------------------------------------------
    
    // get the public keys of all the participants
    let mut tn_pks = get_public_keys(&participant_org_certs);
    let mut wn_pks = get_public_keys(&witness_org_certs);
    let mut participant_pks = Vec::new();
    participant_pks.append(&mut tn_pks); participant_pks.append(&mut wn_pks);

    if !organization_id.identity.check_avg_participants(&participant_pks){
        return Ok(None);
    }


    //--------------------------------------------------------------
    // ORGANIZATION SENDS KEYLOAD
    //--------------------------------------------------------------


    // participants process the channel announcement
    println!("Participants subscribe to channel if not already subscribed:");
    let ann_address = Address::try_from_bytes(&announcement_link.to_bytes())?;
    for i in 0..participant_clients.len() {
        participant_clients[i].receive_announcement(&ann_address).await?;
        let subscribe_msg = participant_clients[i].send_subscribe(&ann_address).await?;
        let sub_result = organization_id.identity.channel_client.receive_subscribe(&subscribe_msg).await;

        // either the subscribe works and the program continues, or it doesnt because
        // the author already has the tn as a subscriber and the program continues
        match sub_result {
            Ok(()) => {println!("-- Participant {} is now subscribed", i);},
            Err(_) => {},
        };
    }
    for i in 0..witness_clients.len() {
        witness_clients[i].receive_announcement(&ann_address).await?;
        let subscribe_msg = witness_clients[i].send_subscribe(&ann_address).await?;
        let sub_result = organization_id.identity.channel_client.receive_subscribe(&subscribe_msg).await;

        match sub_result {
            Ok(()) => {println!("-- Witness {} is now subscribed", i);},
            Err(_) => {},
        };
    }
    println!("");

    println!("Organization sends keyload message to these clients:");
    let (keyload_a_link, _seq_a_link) =
    organization_id.identity.channel_client.send_keyload_for_everyone(&announcement_link).await?;
    println!("-- Keyload sent\n");

    //--------------------------------------------------------------
    // WITNESSES GENERATE SIGS
    //--------------------------------------------------------------

    println!("Witnesses generate their signatures:");
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
    println!("-- Witness signatures generated\n");

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

    println!("Participants generate their signatures:");
    let mut participant_sigs: Vec<interaction_sig::InteractionSig> = Vec::new();
    for i in 0..participant_clients.len() {
        let multibase_pub = MethodData::new_multibase(participant_clients[i].get_public_key());
        let channel_pk_as_multibase: String;
        if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
            channel_pk_as_multibase = mbpub;
        }
        else {
            panic!("Could not encode public key as multibase")
        }
        let sig = generate_sigs::generate_participant_sig(
            contract.clone(),
            channel_pk_as_multibase,
            participant_did_kp[i].clone(),
            WitnessUsers(witnesses.clone()),
            interaction_sig::ArrayOfWnSignituresBytes(witness_sigs_bytes.clone()),
            participant_org_certs[i].clone(),
            DEFAULT_TIMEOUT
        )?;
        participant_sigs.push(sig);
    }
    println!("-- Participant signatures generated\n");

    //--------------------------------------------------------------
    // INITIATING TN, HAVING REVEIVED THE SIGNATURES, 
    // BUILD FINAL TRANSACTION (TN = TRANSACTING NODE)
    //--------------------------------------------------------------

    println!("Initiating participant generates InteractionMessage:");
    let interaction_msg = Message::InteractionMsg {
        contract: contract.clone(),
        witnesses: WitnessUsers(witnesses.clone()),
        witness_sigs: ArrayOfWnSignitures(witness_sigs.clone()),
        witness_sigs: ArrayOfIntSignitures(participant_sigs.clone()),
    };
    println!("-- InteractionMessage generated");
    
    //--------------------------------------------------------------
    // INITIATING TN SENDS THE INTERACTION MESSAGE
    //--------------------------------------------------------------

    // serialise the tx
    let mut tx_msg_str = serde_json::to_string(&interaction_msg)?; 
    tx_msg_str = workaround_channel_bug(run, tx_msg_str);
    let tx_message = vec![
        tx_msg_str
    ];
    println!("-- InteractionMessage serialized\n");


    // TN_A sends the interaction
    println!("Initiating participant sends InteractionMessage:");
    let mut prev_msg_link = keyload_a_link;
    sync_all(&mut participant_clients).await?;
    sync_all(&mut witness_clients).await?;
    let (msg_link, _) = participant_clients[0].send_signed_packet(
        &prev_msg_link,
        &Bytes(tx_message[0].as_bytes().to_vec()),
        &Bytes::default(),
    ).await?;
    println!("-- InteractionMessage sent. ID: {}, tangle index: {:#}\n", msg_link, msg_link.to_msg_index());
    prev_msg_link = msg_link;

    //--------------------------------------------------------------
    // THE EVENT IN QUESTION ON THE CONTRACT PLAYS OUT
    // (WE GENERATE THE OUTCOME AS PART OF THE SIMULATION)
    //--------------------------------------------------------------


    // Dishonest participants still want to get compensated, but are rellying
    // on lazy (or colluding) witnesses for compensation to be more likely. Reason
    // being, the counterparty may still compensate them even if they act dishonestly,
    // but only if the witnesses side with the dishonest node, thus jepordising the 
    // the conterparties trust score.
    println!("Assigning tranascting nodes as (dis)honest according to their reliability:");
    let honest_tranascting_ids = get_honest_users(participant_reliablity);
    println!("Assigning witnesses as (dis)honest according to their reliability:");
    let honest_witness_ids = get_honest_users(witness_reliability);

    // A vector of vectors, the inner a list of the outcomes per participant from
    // the witnesses point of view.
    println!("Witnesses decide on the outcome:");
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
                println!("-- Witnesses {} responds honestly about participant {}", i, j);
            } else {
                outcomes[i].push(lazy_outcome(&lazy_method));
                println!("-- Witnesses {} responds dishonestly about participant {}", i, j);
            }
        }
    }
    println!("");

    //--------------------------------------------------------------
    // WITNESSES SEND THEIR STATMENTS
    //--------------------------------------------------------------

    println!("Witnesses generate and send their witness statements:");
    for i in 0..witness_clients.len(){
        // WN's prepares their statement
        let wn_statement = Message::WitnessStatement {
            outcome: Outcome::ExchangeApplication(outcomes[i].clone())
        };
        let mut wn_statement_string = serde_json::to_string(&wn_statement)?;

        wn_statement_string = workaround_channel_bug(run, wn_statement_string);
        let witness_message = vec![
            wn_statement_string
        ];

        // WN sends their witness statement
        sync_all(&mut participant_clients).await?;
        sync_all(&mut witness_clients).await?;
        let (msg_link, _) = witness_clients[i].send_signed_packet(
            &prev_msg_link,
            &Bytes(witness_message[0].as_bytes().to_vec()),
            &Bytes::default(),
        ).await?;
        println!("-- Witness {} sent statement: ID: {}, tangle index: {:#}", i, msg_link, msg_link.to_msg_index());
        prev_msg_link = msg_link;
    }
    println!("");

    //--------------------------------------------------------------
    // THE PARTICIPANTS READ THE STATEMENTS AND DECIDE TO COMPENSATE
    // OR NOT (NOT WOULD IN PRINCIPAL BE A DISHONEST CHOICE)
    //--------------------------------------------------------------

    // TODO - add read and choice

    println!("Participants send compensation:");
    for i in 0..participant_clients.len(){

        // TODO - certain TNs need to compensate other TNs

        // TN prepares the compensation transaction 
        let payments_tn_a = vec![
            //"tn_b: 0.1".to_string(),
            "wn_a: 0.01".to_string(),
            "wn_b: 0.01".to_string()
        ];
        let compensation_msg = CompensationMsg {
            payments: payments_tn_a
        };

        // because CompensationMsg is application specific, it must be wrapped
        let wrapped_compensation_msg = Message::ApplicationMsg(ApplicationMsg::ExchangeApplication(compensation_msg));

        let mut compensation_msg_str = serde_json::to_string(&wrapped_compensation_msg)?;

        compensation_msg_str = workaround_channel_bug(run, compensation_msg_str);
        let compensation_tx = vec![
            compensation_msg_str
        ];

        // TN sends the compensation transaction
        sync_all(&mut participant_clients).await?;
        sync_all(&mut witness_clients).await?;
        let (msg_link, _) = participant_clients[i].send_signed_packet(
            &prev_msg_link,
            &Bytes(compensation_tx[0].as_bytes().to_vec()),
            &Bytes::default(),
        ).await?;
        println!("-- Participant {} sent compensation: ID: {}, tangle index: {:#}", i, msg_link, msg_link.to_msg_index());
        prev_msg_link = msg_link;
    }
    println!("");
    
    return Ok(Some((honest_tranascting_ids, honest_witness_ids)));
}

pub fn workaround_channel_bug(run: usize, tx: String) -> String {
    let mut st = format!("{}", run);
    st.push_str(&tx);
    return st;
}