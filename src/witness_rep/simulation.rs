use crate::witness_rep::{
    iota_did::create_and_upload_did::{create_n_dids, Key, RunMode},
    interaction::{generate_contract, generate_sigs},
    interaction::{
        interaction::{transact, LazyMethod},
        participant::{
            ParticipantIdentity, OrganizationIdentity,
            IdInfo, ReliabilityMap, Identity, get_index_org_with_pubkey}
    },
    utility::{verify_tx, read_msgs, extract_msgs},
};

use trust_score_generator::trust_score_generators::{
   trivial_tsg::tsg_organization,
   utility::parse_messages
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
use std::collections::BTreeSet;
use std::collections::HashSet;
use std::collections::HashMap;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::fs;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

pub const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
pub const DEFAULT_DURATION: u32 = 60*60*24*365; // 1 year

#[derive(Serialize, Clone, Debug)]
pub struct SimulationConfig {
    pub node_url: String,
    pub num_participants: usize,
    pub average_proximity: f32,
    pub witness_floor: usize,
    pub runs: usize,
    pub reliability: Vec<f32>,
    pub reliability_threshold: Vec<f32>,
    pub default_reliability: Vec<f32>,
    pub organizations: Vec<usize>,
}

// For now this simulation is capturing the abstract scenario where the initiating participant wishes 
// to informally buy something from somebody nearby. However, not all people around them are particpants
// of the system he uses. Therefore, the average_proximity paramater is included. This  represents the
// chance a participant being in range of some other participant.
// 
// Params:
//      - average_proximity: [0,1], 1 meaning all participants are in range
//      - witness_floor: the minimum number of witnesses in a interaction
//      - runs: the number of iterations of the simulations
//      - reliability: an array assigning a reliability score to participants at the respective indices
//      - organizations: an array assigning a organization to participants at the respective indices
pub async fn simulation(
    sc: SimulationConfig
) -> Result<String> {

    if sc.reliability.len() != sc.num_participants {
        panic!("Number of elements in 'reliability' parameter must equal the num_participants!");
    } else if sc.reliability.len() != sc.num_participants {
        panic!("Number of elements in 'organizations' parameter must equal the num_participants!");
    }

    let mut rand_gen = rand::thread_rng();

    let time: DateTime<Utc> = Utc::now();
    let folder_name = format!("./runs/Emmulation run {:?}", time);
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
    
    //--------------------------------------------------------------
    // RUN SIMULATION
    //--------------------------------------------------------------

    // organizations (acting as authors) create the channels
    for i in 0..organizations.len(){
        println!("Creating the channel for oranization {}:", i);
        let announcement_link = organizations[i].identity.channel_client.send_announce().await?;
        let ann_link_string = announcement_link.to_string();
        println!(
            "-- Announcement Link: {} Tangle Index: {:#}\n",
            ann_link_string, announcement_link.to_msg_index()
        );

        // change the organization struct to include their channel announcement
        organizations[i].ann_msg = Some(ann_link_string);
    }


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
        // run the iteration
        let ran = simulation_iteration(
            organizations,
            participants,
            sc.average_proximity,
            sc.witness_floor,
            lazy_methods[i].clone(),
            &sc.node_url,
            &format!("output_{}", i),
            &mut rand_gen,
            i,
            folder_name.clone()
        ).await?;

        if !ran {
            println!("FAILED TO RUN")
        }

        participants = reset_clients(participants, client.clone())?;
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


// Runs a single iteration of a simualtion
pub async fn simulation_iteration(
    organizations: &mut Vec<OrganizationIdentity>,
    mut participants: &mut Vec<ParticipantIdentity>,
    average_proximity: f32,
    witness_floor: usize,
    lazy_method: LazyMethod,
    node_url: &str,
    output_name: &str,
    rand_gen: &mut rand::prelude::ThreadRng,
    run: usize,
    folder_name: String
) -> Result<bool> {

    //--------------------------------------------------------------
    // GENERATE GROUPS OF TRANSACATING NODES AND WITNESSES
    //--------------------------------------------------------------

    let gen_op = generate_trans_and_witnesses(
        &mut participants,
        average_proximity,
        witness_floor,
        rand_gen,
        100,
        true
    )?;

    let (mut transacting_clients, mut witness_clients) = match gen_op{
        None => {
            return Ok(false);
        },
        Some(x) => x
    };

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
    // PERFORM THE INTERACTION WITH CONTRACT
    //--------------------------------------------------------------

    let (tn_honesty, wn_honesty) = transact(
        contract,
        &mut transacting_clients,
        &mut witness_clients,
        &mut organizations[org_index],
        lazy_method,
        run,
    ).await?;

    // put the particpants back into the original array
    participants.append(&mut witness_clients);
    participants.append(&mut transacting_clients);

    //--------------------------------------------------------------
    // VERIFY THE INTERACTION AND SAVE THE OUTPUT TO FILE
    //--------------------------------------------------------------

    // verify the interaction
    let ann_msg = &organizations[org_index].ann_msg.as_ref().unwrap();
    let org_seed = &organizations[org_index].seed;
    let branches = verify_tx::WhichBranch::LastBranch;
    let channel_msgs = read_msgs::read_msgs(node_url, ann_msg, org_seed).await?;
    let (verified, msgs, pks) = verify_tx::verify_txs(channel_msgs, branches).await?;

    if !verified {
        panic!("One of the messages could not be verified");
    }

    // for each message
    let mut output: String = String::new();
    let info = format!("TN honesty {:?}\nWN honesty: {:?}\n\n", tn_honesty, wn_honesty);
    output.push_str(&info);
    for i in 0..msgs.len() {
        // print the message and then the id_info of the sender
        let msg = format!("Message {:?}\n", msgs[i]);
        let pk = format!("Channel pubkey: {:?}\n\n", pks[i]);
        output.push_str(&msg);
        output.push_str(&pk);
    }
    let file_name = format!("{}/{}", &folder_name, &output_name);
    fs::write(file_name, output).expect("Unable to write file");

    //--------------------------------------------------------------
    // ALL PARTICIPANTS NOW UPDATE THEIR RELIABILITY SCORES BY
    // PROCESSING THE LATEST INTERACTION
    //--------------------------------------------------------------

    // participants update their reliability scores of each other
    let channel_msgs = read_msgs::read_msgs(node_url, ann_msg, org_seed).await?;
    let branch_msgs = extract_msgs::extract_msg(channel_msgs, verify_tx::WhichBranch::LastBranch);
    //println!("HHHHEERREEE: {:?}", branch_msgs);
    let parsed_msgs = parse_messages::parse_messages(&branch_msgs)?;
    for part in participants.into_iter() {
        let (tn_verdicts, wn_verdicts) = tsg_organization(
            parsed_msgs.clone(),
            part.id_info.org_cert.org_pubkey.clone(),
            0.5
        );

        // add the new verdicts to the reliability map
        part.update_reliability(tn_verdicts.clone());
        part.update_reliability(wn_verdicts.clone());

        //println!("tn_verdicts: {:?}", tn_verdicts);
        //println!("wn_verdicts: {:?}\n", wn_verdicts);
    }

    return Ok(true);
}

// Generates the transacting nodes and the witnesses for the next simulation.
// Will return None if no witnesses can be found after max_tries
pub fn generate_trans_and_witnesses(
    participants: &mut Vec<ParticipantIdentity>,
    average_proximity: f32,
    witness_floor: usize,
    rand_gen: &mut rand::prelude::ThreadRng,
    max_tries: usize,
    print: bool
) -> Result<Option<(Vec<ParticipantIdentity>,Vec<ParticipantIdentity>)>> {

    let mut transacting_clients: Vec<ParticipantIdentity> = Vec::new();
    let mut witness_clients: Vec<ParticipantIdentity> = Vec::new();

    // we select the initiating transacting participant as the first participant
    transacting_clients.push(participants.remove(0));
    
    // The initiating transacting participant searches for another to transact with.
    // Using mod, this section will only finish when one is found, representing the start
    // of the process
    let mut count = 0;
    loop {
        if average_proximity > rand_gen.gen() {
            transacting_clients.push(participants.remove(count % participants.len()));
            break;
        }
        count = count + 1;
    }

    // The transacting participants now search for witnesses and combine their results.
    // Each iteration of the upper loop is one of the transacting nodes searching for
    // witnesses. We must work with indexes instead of actual objects to removing potential
    // witnesses from the list for transacting nodes of indices larger than 0
    if print{
        println!("Selecting participants to be transacting nodes and witnesses:");
    }
    let mut main_set_of_witnesses: BTreeSet<usize> = BTreeSet::new();
    for i in 0.. {
        if i >= max_tries {
            participants.append(&mut transacting_clients);
            return Ok(None);
        }

        let mut tn_witnesses_lists: Vec<Vec<usize>> = Vec::new();
        for i in 0..transacting_clients.len(){
            if print{
                println!("-- Transacting node {} is finding witnesses:", i);
            }
            let mut tn_witnesses: Vec<usize> = Vec::new();

            for j in 0..participants.len(){
                let rand: f32 = rand_gen.gen();
                if print{
                    println!("---- Trying participant {}. Rand={}. Avg proximity={}", j, rand, average_proximity);
                }
                if average_proximity > rand {
                    if print{
                        println!("---- Checking participant {}'s reliability", j);
                    }
                    let potential_wn_pk = participants[j].id_info.org_cert.client_pubkey.clone();
                    if transacting_clients[i].check_participant(&potential_wn_pk){
                        tn_witnesses.push(j);
                        if print{
                            println!("------ Participant {} added", j);
                        }
                    }
                }
            }

            if print{
                println!("---- Found witnesses at indices: {:?}\n", tn_witnesses);
            }
            tn_witnesses_lists.push(tn_witnesses);
        }

        // The transacting participants combine their witnesses, and check if there are enough.
        // Using BTreeSet because it is ordered
        for witness in tn_witnesses_lists[0].clone(){
            main_set_of_witnesses.insert(witness);
        }

        for i in 1..tn_witnesses_lists.len() {
            let mut set_of_witnesses: BTreeSet<usize> = BTreeSet::new();
            for witness in tn_witnesses_lists[i].clone(){
                set_of_witnesses.insert(witness);
            }
            main_set_of_witnesses = main_set_of_witnesses.intersection(&set_of_witnesses).cloned().collect();
        }

        println!("-- Final list of witness indices: {:?}", main_set_of_witnesses);

        if main_set_of_witnesses.len() >= witness_floor {
            break;
        }
    }

    // convert indices into objects (as it is ordered, we can account for
    // the changing indices)
    for (i, witness) in main_set_of_witnesses.iter().enumerate() {
        witness_clients.push(participants.remove(*witness - i))
    }

    return Ok(Some((transacting_clients, witness_clients)));
}

pub fn reset_clients(
    participants: &mut Vec<ParticipantIdentity>,
    client: Client
) -> Result<&mut Vec<ParticipantIdentity>> {
    for i in 0..participants.len(){
        let new_client = Subscriber::new(&participants[i].seed, client.clone());
        participants[i].channel_client = new_client;
    }
    return Ok(participants);
}