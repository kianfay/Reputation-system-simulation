use crate::witness_rep::{
    iota_did::create_and_upload_did::{create_n_dids, Key, RunMode},
    transaction::{generate_contract, generate_sigs},
    transaction::{
        transaction::{LazyMethod},
        quick_transaction::quick_transact,
        participant::{
            ParticipantIdentity, OrganizationIdentity,
            IdInfo, ReliabilityMap, Identity, get_index_org_with_pubkey}
    },
    simulation::{SimulationConfig, generate_trans_and_witnesses}
};

use trust_score_generator::trust_score_generators::trivial_tsg::tsg_organization;

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
    sc: SimulationConfig
) -> Result<String> {

    if sc.reliability.len() != sc.num_participants {
        panic!("Number of elements in 'reliability' parameter must equal the num_participants!");
    } else if sc.reliability.len() != sc.num_participants {
        panic!("Number of elements in 'organizations' parameter must equal the num_participants!");
    }

    let mut rand_gen = rand::thread_rng();
    
    let time: DateTime<Utc> = Utc::now();
    let folder_name = format!("./runs/Quick emmulation run {:?}", time);
    println!("{}", folder_name);
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
    let orgs_set: HashSet<&usize> = HashSet::from_iter(sc.organizations.iter());
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
        let reliability_map: ReliabilityMap = HashMap::new();

        let org_id: Identity<Author<Client>> = Identity{
            channel_client: on,
            seed: String::from(" "),
            id_info: IdInfo {
                did_key: sec,
                reliability: sc.reliability[i],
                org_cert: generate_sigs::generate_org_cert(pubkey, &repeat_kp, DEFAULT_DURATION)?
            },
            reliability_map: reliability_map,
            reliability_threshold: sc.reliability_threshold[i],
            default_reliability: sc.default_reliability[i]
        };

        let org_id_with_announcement = OrganizationIdentity{
            identity: org_id,
            ann_msg: None,
            seed: String::from(seed)
        };
        organizations.push(org_id_with_announcement);
    }


    //--------------------------------------------------------------
    // CREATE PARTICIPANTS FOR SIMULATION
    // (MORE DETAILS IN ALL_IN_ONE_TRANSACTION.RS)
    //--------------------------------------------------------------

    // create Decentalised Ids (for now, none needed for the organization)
    let did_details = create_n_dids(sc.num_participants, RunMode::Testing).await?;
    
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
    let mut participants: &mut Vec<ParticipantIdentity> = &mut Vec::new();
    for i in 0..sc.num_participants{
        let name = format!("Participant {}", i);
        let tn = Subscriber::new(&name, client.clone());
        let org_kp = &org_kp_map[&sc.organizations[i]];
        let part_did_pk = generate_sigs::get_multibase(&part_did_kps[i]);
        let reliability_map: ReliabilityMap = HashMap::new();

        let id = ParticipantIdentity {
            channel_client: tn,
            seed: name,
            id_info: IdInfo {
                did_key: part_did_secret[i],
                reliability: sc.reliability[i],
                org_cert: generate_sigs::generate_org_cert(part_did_pk.clone(), org_kp, DEFAULT_DURATION)?
            },
            reliability_map: reliability_map,
            reliability_threshold: sc.reliability_threshold[i],
            default_reliability: sc.default_reliability[i]
        };
        participants.push(id);

        let part_entry = format!("{}: {}\n", part_did_pk, sc.reliability[i]);
        output.push_str(&part_entry);
    }

    let file_name = format!("{}/start_reliability.txt", &folder_name);
    fs::write(file_name, output).expect("Unable to write file");

    // generate the lazy methods (currenlty the first half of the runs are 
    // 'constant true' and the second half are 'random')
    println!("Generating lazy methods:");
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
    println!("-- Lazy methods to be used: {:?}\n", lazy_methods);

    for i in 0..sc.runs {
        println!("\n\n\n---------------------STARTING RUN {}---------------------", i);
        //--------------------------------------------------------------
        // GENERATE GROUPS OF TRANSACATING NODES AND WITNESSES
        //--------------------------------------------------------------

        let (mut transacting_clients, mut witness_clients) = generate_trans_and_witnesses(&mut participants,
            sc.average_proximity,
            sc.witness_floor,
            &mut rand_gen)?;

        //--------------------------------------------------------------
        // GET THE ORGANIZATION OF THE INITIATING TRANSACTING NODE [0]
        //--------------------------------------------------------------

        // get orgs' pubkey and find the org with that pubkey
        let init_tn_org_pk = &transacting_clients[0].id_info.org_cert.org_pubkey;
        let org_index = get_index_org_with_pubkey(&organizations, init_tn_org_pk);
        println!("\nRun under organization {}\n", organizations[org_index].identity.id_info.org_cert.client_pubkey);

        //--------------------------------------------------------------
        // GENERATE CONTRACT
        //--------------------------------------------------------------

        println!("Generating contract:");
        let contract = generate_contract::generate_contract(&mut transacting_clients)?;
        println!("-- Contract generated\n");

        //--------------------------------------------------------------
        // PERFORM THE TRANSACTION WITH CONTRACT
        //--------------------------------------------------------------

        let (tn_honesty, wn_honesty, msgs) = quick_transact(
            contract,
            &mut transacting_clients,
            &mut witness_clients,
            &mut organizations[org_index],
            lazy_methods[i].clone(),
            i,
        ).await?;

        // put the particpants back into the original array
        participants.append(&mut witness_clients);
        participants.append(&mut transacting_clients);

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
            );

            // add the new verdicts to the reliability map
            part.update_reliability(tn_verdicts.clone());
            part.update_reliability(wn_verdicts.clone());

            //println!("tn_verdicts: {:?}", tn_verdicts);
            //println!("wn_verdicts: {:?}\n", wn_verdicts);
        }  
    }

    // write all of the reliability maps to file, next to their did public key
    let mut output: String = String::new();
    for part in participants {
        let pk = format!("{}\n", part.id_info.org_cert.client_pubkey);
        let map = format!("{}\n\n", part.get_reliability_scores_string());
        output.push_str(&pk);
        output.push_str(&map);
    }
    let file_name = format!("{}/reliability_maps.txt", &folder_name);
    fs::write(file_name, output).expect("Unable to write file");

    return Ok(folder_name);
}