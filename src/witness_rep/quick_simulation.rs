use crate::witness_rep::{
    iota_did::create_and_upload_did::{create_n_dids, Key, RunMode},
    implementation::{generate_contract, generate_sigs},
    implementation::{
        interaction::{LazyMethod},
        quick_interaction::quick_interaction,
        user_and_organization::{
            UserIdentity, OrganizationIdentity,
            IdInfo, get_index_org_with_pubkey
        }
    },
    simulation::{SimulationConfig, generate_participants_and_witnesses}
};

use wb_reputation_system::{
    trust_score_generators::{
        exchange_application_tsg::trivial_tsg::tsg_organization
    },
    data_types::identity::identity::{
        Identity ,ReputationMap
    }
};

use iota_streams::{
    app::transport::tangle::client::Client,
    app_channels::api::tangle::{
        Author, ChannelType, Subscriber
    },
    app_channels::Tangle,
    core::Result
};
use identity::{
    crypto::KeyPair,
};

use rand::Rng;
use std::collections::HashSet;
use std::collections::HashMap;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::fs;
use chrono::prelude::*;

pub const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
pub const DEFAULT_DURATION: u32 = 60*60*24*365; // 1 year

pub async fn quick_simulation(
    sc: SimulationConfig,
    print: bool
) -> Result<(String, bool)> {

    if sc.reliability.len() != sc.num_users {
        panic!("Number of elements in 'reliability' parameter must equal the num_users!");
    } else if sc.reliability.len() != sc.num_users {
        panic!("Number of elements in 'organizations' parameter must equal the num_users!");
    }

    let mut rand_gen = rand::thread_rng();
    
    let time: DateTime<Utc> = Utc::now();
    let folder_name = format!("./runs/Quick emmulation run {:?}", time);
    if print {
        println!("{}", folder_name);
    }
    fs::create_dir(&folder_name)?;

    
    let file_name = format!("{}/sim_parameters.txt", &folder_name);
    let output = serde_json::to_string(&sc)?;
    fs::write(file_name, output).expect("Unable to write file");


    //--------------------------------------------------------------
    //--------------------------------------------------------------
    //  CREATE ORGANIZATIONS WHICH ACT AS AN OVERLAY FOR PARTICIPANTS
    //--------------------------------------------------------------
    
    let client = Client::new_from_url(&sc.node_url);

    // we find the set of organizations
    let orgs_set: HashSet<&usize> = HashSet::from_iter(sc.user_organizations.iter());
    let orgs: Vec<&usize> = orgs_set.into_iter().collect();

    // in their simplest form, an organization can be represented by
    // a keypair, so we assign one to each organization
    let org_did_details = create_n_dids(orgs.len(), RunMode::Testing).await?;

    // we create a mapping of organization index to public key and 
    // create an OrganizationIdentity object for each organization
    let mut org_kp_map: HashMap<usize, KeyPair> = HashMap::new();
    let organizations: &mut Vec<OrganizationIdentity> = &mut Vec::new();
    let mut i = 0;
    for (_, (kp, (_,sec)), _) in org_did_details {
        org_kp_map.insert(orgs[i].clone(), kp);
        i += 1;

        let seed: &str = &(0..81)
        .map(|_| {
            ALPH9
                .chars()
                .nth(rand_gen.gen_range(0, 27))
                .unwrap()
        })
        .collect::<String>();

        let on: Author<Tangle> = Author::new(seed, ChannelType::MultiBranch, client.clone());
        let repeat_kp = KeyPair::try_from_ed25519_bytes(&sec)?;
        let pubkey =  generate_sigs::get_multibase(&repeat_kp);
        let reputation_map: ReputationMap = HashMap::new();

        let org_id: Identity<Author<Client>, IdInfo> = Identity{
            channel_client: on,
            id_info: IdInfo {
                seed: None,
                did_key: sec,
                reliability: None,
                org_cert: generate_sigs::generate_org_cert(pubkey, &repeat_kp, DEFAULT_DURATION)?
            },
            reputation_map: reputation_map,
            user_reputation_threshold: sc.user_reputation_threshold[i],
            user_default_reputation: sc.user_default_reputation[i]
        };

        let org_id_with_announcement = OrganizationIdentity{
            identity: org_id,
            ann_msg: Some(String::from("placeholder"))
        };
        organizations.push(org_id_with_announcement);
    }


    //--------------------------------------------------------------
    // CREATE PARTICIPANTS FOR SIMULATION
    // (MORE DETAILS IN ALL_IN_ONE_TRANSACTION.RS)
    //--------------------------------------------------------------

    // create Decentalised Ids (for now, none needed for the organization)
    let did_details = create_n_dids(sc.num_users, RunMode::Testing).await?;
    
    let part_did_secret : Vec<Key> = did_details
                                            .iter()
                                            .map(|(_, (_,(_, privkey)), _)| *privkey)
                                            .collect();
    
    let part_did_kps : Vec<&KeyPair> = did_details
                                            .iter()
                                            .map(|(_, (kp,_), _)| kp)
                                            .collect();

    // create channel subscriber instances
    let mut output: String = String::new();
    let mut participants: &mut Vec<UserIdentity> = &mut Vec::new();
    for i in 0..sc.num_users{
        let name = format!("Participant {}", i);
        let tn = Subscriber::new(&name, client.clone());
        let org_kp = &org_kp_map[&sc.user_organizations[i]];
        let part_did_pk = generate_sigs::get_multibase(&part_did_kps[i]);
        let reputation_map: ReputationMap = HashMap::new();

        // by adding the duration to the current time, we get the point of timeout
        let cur_time = time.timestamp() as u32;
        let timeout = cur_time + DEFAULT_DURATION;

        let id = UserIdentity {
            channel_client: tn,
            id_info: IdInfo {
                seed: Some(name),
                did_key: part_did_secret[i],
                reliability: Some(sc.reliability[i]),
                org_cert: generate_sigs::generate_org_cert(part_did_pk.clone(), org_kp, timeout)?
            },
            reputation_map: reputation_map,
            user_reputation_threshold: sc.user_reputation_threshold[i],
            user_default_reputation: sc.user_default_reputation[i]
        };
        participants.push(id);

        let part_entry = format!("{}: {}\n", part_did_pk, sc.reliability[i]);
        output.push_str(&part_entry);
    }

    let file_name = format!("{}/start_reliability.txt", &folder_name);
    fs::write(file_name, output).expect("Unable to write file");

    // generate the lazy methods (currenlty the first half of the runs are 
    // 'constant true' and the second half are 'random')
    if print {
        println!("Generating lazy methods:");
    }
    let lazy_methods: Vec<LazyMethod> = (0..sc.runs)
        .map(|_| {
            match rand_gen.gen_range(0,3) {
                0 => LazyMethod::Constant(true),
                1 => LazyMethod::Constant(false),
                2 => LazyMethod::Random,
                _ => panic!("Random number generator failure")
            }
        }).collect::<Vec<LazyMethod>>()
        .try_into().expect("wrong size iterator");
    if print {
        println!("-- Lazy methods to be used: {:?}\n", lazy_methods);
    }

    let mut ran_fully = true;
    for i in 0..sc.runs {
        println!("\n\n\n---------------------STARTING RUN {}---------------------", i);
        //--------------------------------------------------------------
        // GENERATE GROUPS OF TRANSACATING NODES AND WITNESSES
        //--------------------------------------------------------------

        let gen_op = generate_participants_and_witnesses(
            &mut participants,
            sc.average_proximity,
            sc.witness_floor,
            &mut rand_gen,
            100,
            print
        )?;
    
        let (mut participant_clients, mut witness_clients) = match gen_op{
            None => {
                println!("FAILED TO RUN");
                ran_fully = false;
                continue;
            },
            Some(x) => x
        };

        //--------------------------------------------------------------
        // GET THE ORGANIZATION OF THE INITIATING TRANSACTING NODE [0]
        //--------------------------------------------------------------

        // get orgs' pubkey and find the org with that pubkey
        let init_tn_org_pk = &participant_clients[0].id_info.org_cert.org_pubkey;
        let org_index = get_index_org_with_pubkey(&organizations, init_tn_org_pk);
        if print {
            println!("\nRun under organization {}\n", organizations[org_index].identity.id_info.org_cert.client_pubkey);        
        }

        //--------------------------------------------------------------
        // GENERATE CONTRACT
        //--------------------------------------------------------------

        if print {
            println!("Generating contract:");        
        }
        let contract = generate_contract::generate_exchange_contract(
            &mut participant_clients,
            organizations[org_index].ann_msg.clone().unwrap()
        )?;
        if print {
            println!("-- Contract generated\n");
        }

        //--------------------------------------------------------------
        // PERFORM THE INTERACTION WITH CONTRACT
        //--------------------------------------------------------------

        let op_ret = quick_interaction(
            contract,
            &mut participant_clients,
            &mut witness_clients,
            &mut organizations[org_index],
            lazy_methods[i].clone(),
            i,
            print
        ).await?;

        let (tn_honesty, wn_honesty, msgs) = match op_ret {
            None => {
                ran_fully = false;
                continue;
            },
            Some(x) => x
        };

        // put the particpants back into the original array
        participants.append(&mut witness_clients);
        participants.append(&mut participant_clients);

        //--------------------------------------------------------------
        // SAVE THE OUTPUT TO FILE
        //--------------------------------------------------------------

        // for each message
        let mut output: String = String::new();
        let info = format!("TN honesty {:?}\nWN honesty: {:?}\n\n", tn_honesty, wn_honesty);
        output.push_str(&info);
        for i in 0..msgs.len() {
            // print the message and then the id_info of the sender
            let msg = format!("Message {:?}\n\n", msgs[i]);
            output.push_str(&msg);
        }
        let file_name = format!("{}/{}", &folder_name, &format!("output_{}", i));
        fs::write(file_name, output).expect("Unable to write file");

        //--------------------------------------------------------------
        // ALL PARTICIPANTS NOW UPDATE THEIR RELIABILITY SCORES BY
        // PROCESSING THE LATEST TRANSACTION
        //--------------------------------------------------------------

        // participants update their reliability scores of each other
        for part in participants.into_iter() {
            let (tn_verdicts, wn_verdicts) = tsg_organization(
                msgs.clone(),
                part.id_info.org_cert.org_pubkey.clone(),
                0.5
            ).unwrap();

            // add the new verdicts to the reliability map
            part.update_reputation(tn_verdicts.clone());
            part.update_reputation(wn_verdicts.clone());

            //println!("tn_verdicts: {:?}", tn_verdicts);
            //println!("wn_verdicts: {:?}\n", wn_verdicts);
        }  
    }

    // write all of the reliability maps to file, next to their did public key
    let mut output: String = String::new();
    for part in participants {
        let pk = format!("{}\n", part.id_info.org_cert.client_pubkey);
        let map = format!("{}\n\n", part.get_reputation_scores_string());
        output.push_str(&pk);
        output.push_str(&map);
    }
    let file_name = format!("{}/reputation_maps.txt", &folder_name);
    fs::write(file_name, output).expect("Unable to write file");

    return Ok((folder_name, ran_fully));
}