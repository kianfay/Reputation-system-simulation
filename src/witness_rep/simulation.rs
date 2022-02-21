use crate::witness_rep::{
    iota_did::create_and_upload_did::{create_n_dids, Key, RunMode},
    transaction::{generate_contract, generate_sigs},
    transaction::{
        transaction::{transact, LazyMethod},
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
    app::transport::tangle::{TangleAddress, client::Client},
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

pub const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
pub const DEFAULT_DURATION: u32 = 60*60*24*365; // 1 year

pub struct SimulationConfig {
    pub node_url: String,
    pub num_participants: usize,
    pub average_proximity: f32,
    pub witness_floor: usize,
    pub runs: usize,
    pub reliability: Vec<f32>,
    pub reliability_threshold: Vec<f32>,
    pub default_reliability: Vec<f32>,
    pub organizations: Vec<usize>
}

// For now this simulation is capturing the abstract scenario where the initiating participant wishes 
// to informally buy something from somebody nearby. However, not all people around them are particpants
// of the system he uses. Therefore, the average_proximity paramater is included. This  represents the
// chance a participant being in range of some other participant.
// 
// Params:
//      - average_proximity: [0,1], 1 meaning all participants are in range
//      - witness_floor: the minimum number of witnesses in a transaction
//      - runs: the number of iterations of the simulations
//      - reliability: an array assigning a reliability score to participants at the respective indices
//      - organizations: an array assigning a organization to participants at the respective indices
pub async fn simulation(
    sc: SimulationConfig
) -> Result<()> {

    if sc.reliability.len() != sc.num_participants {
        panic!("Number of elements in 'reliability' parameter must equal the num_participants!");
    } else if sc.reliability.len() != sc.num_participants {
        panic!("Number of elements in 'organizations' parameter must equal the num_participants!");
    }

    let mut rand_gen = rand::thread_rng();


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
    let participants: &mut Vec<ParticipantIdentity> = &mut Vec::new();
    for i in 0..sc.num_participants{
        let name = format!("Participant {}", i);
        let tn = Subscriber::new(&name, client.clone());
        let org_kp = &org_kp_map[&sc.organizations[i]];
        let part_did_pk = generate_sigs::get_multibase(&part_did_kps[i]);
        let reliability_map: ReliabilityMap = HashMap::new();

        let id = ParticipantIdentity {
            channel_client: tn,
            id_info: IdInfo {
                did_key: part_did_secret[i],
                reliability: sc.reliability[i],
                org_cert: generate_sigs::generate_org_cert(part_did_pk, org_kp, DEFAULT_DURATION)?
            },
            reliability_map: reliability_map,
            reliability_threshold: sc.reliability_threshold[i],
            default_reliability: sc.default_reliability[i]
        };
        participants.push(id);
    }
    
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
        // run the iteration
        simulation_iteration(
            organizations,
            participants,
            sc.average_proximity,
            sc.witness_floor,
            lazy_methods[i].clone(),
            &sc.node_url,
            &format!("output_{}", i),
            &mut rand_gen
        ).await?;
    }
    
    return Ok(());
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
    rand_gen: &mut rand::prelude::ThreadRng
) -> Result<()> {

    //--------------------------------------------------------------
    // GENERATE GROUPS OF TRANSACATING NODES AND WITNESSES
    //--------------------------------------------------------------

    let (mut transacting_clients, mut witness_clients) = generate_trans_and_witnesses(&mut participants,
        average_proximity,
        witness_floor,
        rand_gen)?;

    //--------------------------------------------------------------
    // GET THE ORGANIZATION OF THE INITIATING TRANSACTING NODE [0]
    //--------------------------------------------------------------

    // get orgs' pubkey and find the org with that pubkey
    let init_tn_org_pk = &transacting_clients[0].id_info.org_cert.org_pubkey;
    let org_index = get_index_org_with_pubkey(&organizations, init_tn_org_pk);


    //--------------------------------------------------------------
    // GENERATE CONTRACT
    //--------------------------------------------------------------

    println!("Generating contract:");
    let contract = generate_contract::generate_contract(&mut transacting_clients)?;
    println!("-- Contract generated\n");

    //--------------------------------------------------------------
    // PERFORM THE TRANSACTION WITH CONTRACT
    //--------------------------------------------------------------

    transact(
        contract,
        &mut transacting_clients,
        &mut witness_clients,
        &mut organizations[org_index],
        lazy_method
    ).await?;

    // put the particpants back into the original array
    participants.append(&mut transacting_clients);
    participants.append(&mut witness_clients);


    //--------------------------------------------------------------
    // VERIFY THE TRANSACTION AND SAVE THE OUTPUT TO FILE
    //--------------------------------------------------------------

    // verify the transaction
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
    for i in 0..msgs.len() {
        // print the message and then the id_info of the sender
        let msg = format!("Message {:?}\n", msgs[i]);
        let pk = format!("Channel pubkey: {:?}\n\n", pks[i]);
        output.push_str(&msg);
        output.push_str(&pk);
    }
    fs::write(output_name, output).expect("Unable to write file");

    //--------------------------------------------------------------
    // ALL PARTICIPANTS NOW UPDATE THEIR RELIABILITY SCORES BY
    // PROCESSING THE LATEST TRANSACTION
    //--------------------------------------------------------------

    // participants update their reliability scores of each other
    let channel_msgs = read_msgs::read_msgs(node_url, ann_msg, org_seed).await?;
    let branch_msgs = extract_msgs::extract_msg(channel_msgs, verify_tx::WhichBranch::LastBranch);
    let parsed_msgs = parse_messages::parse_messages(&branch_msgs[0])?;
    for part in participants.into_iter() {
        let (tn_verdicts, wn_verdicts) = tsg_organization(
            parsed_msgs.clone(),
            part.id_info.org_cert.org_pubkey.clone(),
            0.5
        );
        println!("tn_verdicts: {:?}", tn_verdicts);
        println!("wn_verdicts: {:?}\n", wn_verdicts);
    }

    return Ok(());
}

// Generates the transacting nodes and the witnesses for the next simulation
pub fn generate_trans_and_witnesses(
    participants: &mut Vec<ParticipantIdentity>,
    average_proximity: f32,
    witness_floor: usize,
    rand_gen: &mut rand::prelude::ThreadRng
) -> Result<(Vec<ParticipantIdentity>,Vec<ParticipantIdentity>)> {

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
    let tn_witnesses_lists: &mut Vec<Vec<usize>> = &mut Vec::new();

    println!("Selecting participants to be transacting nodes and witnesses:");
    for i in 0..transacting_clients.len(){
        println!("-- Transacting node {} is finding witnesses:", i);
        let mut tn_witnesses: Vec<usize> = Vec::new();

        for j in 0..participants.len(){
            let rand: f32 = rand_gen.gen();
            println!("---- Trying participant {}. Rand={}. Avg proximity={}", j, rand, average_proximity);
            if average_proximity > rand {
                println!("---- Checking participant {}'s reliability", j);
                let potential_wn_pk = participants[j].id_info.org_cert.client_pubkey.clone();
                if transacting_clients[i].check_participant(&potential_wn_pk){
                    tn_witnesses.push(j);
                    println!("------ Participant {} added", j);
                }
            }
        }

        println!("---- Found witnesses at indices: {:?}\n", tn_witnesses);
        tn_witnesses_lists.push(tn_witnesses);
    }

    // The transacting participants combine their witnesses, and check if there are enough.
    // Using BTreeSet because it is ordered
    let mut set_of_witnesses: BTreeSet<&mut usize> = BTreeSet::new();
    for witnesses in tn_witnesses_lists{
        for witness in witnesses{
            set_of_witnesses.insert(witness);
        }
    }

    if set_of_witnesses.len() < witness_floor {
        panic!("Not enough witnesses were generated.")
    }

    // convert indices into objects (as it is ordered, we can account for
    // the changing indices)
    for (i, witness) in set_of_witnesses.iter().enumerate() {
        witness_clients.push(participants.remove(**witness - i))
    }

    return Ok((transacting_clients, witness_clients));
}
