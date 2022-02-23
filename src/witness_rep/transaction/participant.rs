use crate::witness_rep::{
    iota_did::create_and_upload_did::Key,
};

use trust_score_generator::trust_score_generators::{
    data_types::{
        messages::signatures::organization_cert,
        verdict::TxVerdict
    }
};

use iota_streams::{
    app::transport::tangle::{TangleAddress, client::Client},
    app_channels::api::tangle::{Subscriber, Author}
};


use std::collections::HashMap;

// The identity captures the channel client and the associated keypair,
// as well as the keypair associated to the participants DID. It also has their 
// 'reliability', [0,1], an unambiguous/simplistic measure of the honesty of their
// actions. A score of 1 means they are always honest, and 0 means always dishonest.
// A more dishonest participant will more likely to give a either not uphold their
// half of an agreement, or more likely to give a lazy witness statement (i.e. not
// depending on the actual event), or to possibly collude or act with malice to 
// either gain an advantage in monetary or trust score terms (or damage other 
// participant).
pub struct Identity<C> {
    /// The IOTA channels client
    pub channel_client: C,
    /// Information inherent to the participant outside of the simulation
    pub id_info: IdInfo,
    /// A Map from public keys to the perceived reliability of the associated participant
    pub reliability_map: ReliabilityMap,
    /// The minimum trust this participant needs to have to engage with another participant.
    /// For an organization, this is the minimum average reliability of the participants
    /// needed to allow the transaction.
    pub reliability_threshold: f32,
    /// The trust this participant has in a unknown participant
    pub default_reliability: f32
}

impl<C> Identity<C> {
    pub fn update_reliability(&mut self, new_estimates: TxVerdict){
        for estimate in new_estimates.verdicts {
            // inserts the default value if the entry does not yet exist
            self.add_if_not_already(&estimate.did_public_key);

            // remove the old mapping, and then update and re-insert
            let (score, estimate_count) = self.reliability_map.remove(&estimate.did_public_key).unwrap();
            self.reliability_map.insert(estimate.did_public_key, (score + estimate.estimated_reliablility, estimate_count + 1));
        }        
    }

    /// Average out the participants reliability scores and checks if this
    /// passes the reliability threshold
    pub fn check_avg_participants(&mut self, participants: &Vec<String>) -> bool{
        participants
            .iter()
            .for_each(|p| self.add_if_not_already(p)); 
        
        let reliability_sum: f32 = participants
            .iter()
            .map(|p| self.reliability_map.get(p).unwrap())
            .map(|r| self.calculate_score(r))
            .sum();
        return (reliability_sum / participants.len() as f32) >= self.reliability_threshold;
    }

    /// Checks if all participants pass the reliability threshold
    pub fn check_all_participants(&mut self, participants: &Vec<String>) -> bool{
        return participants.iter().all(|p| self.check_participant(p));
    }

    /// Checks if a participant passes the reliability threshold
    pub fn check_participant(&mut self, participant: &str) -> bool{
        self.add_if_not_already(participant); 

        let reliability = self.calculate_score(self.reliability_map.get(participant).unwrap());
        return reliability >= self.reliability_threshold;
    }

    /// Adds an entry into reliability_map if it does not yet exist
    pub fn add_if_not_already(&mut self, participant: &str) {
        self.reliability_map.entry(String::from(participant)).or_insert((0.0, 0));
    }

    // Because the default value in the reliability map is (0.0, 0), a div by zero
    // error must be avoided. Conveniently, (_, 0) means we should use the default value.
    pub fn calculate_score(&self, components: &ReliabilityComponents) -> f32 {
        if components.1 == 0{
            return self.default_reliability;
        }
        return components.0 / components.1 as f32;
    }

    pub fn get_reliability_scores_string(&self) -> String {
        let mut str = String::new();
        self.reliability_map
            .iter()
            .for_each(|(p,r)| {
                let score = self.calculate_score(r);
                let next = format!("{}: {}\n", p, score);
                str.push_str(&next)
            });
        return str;
    }

}



pub type ParticipantIdentity = Identity<Subscriber<Client>>;
pub struct OrganizationIdentity{
    pub identity:  Identity<Author<Client>>,
    pub ann_msg: Option<String>,
    pub seed: String
}

// This is all of the external information about a participant, including their
// decentralised ID and their reliability
pub struct IdInfo {
    pub did_key: Key,
    pub reliability: f32,
    pub org_cert: organization_cert::OrgCert
}

/// Maps public keys to a reliability score
pub type ReliabilityMap = HashMap<String, ReliabilityComponents>;

// Reliability is calculated from reliability estimates. One reliability
// estimate of 0.5 causes the ReliabilityScore = (0.5,1), and the score
// is 0.5/1=0.5. After another estimate of 1.0, the ReliabilityScore = (1.5,2),
// and the score is 1.5/2=0.75
pub type ReliabilityComponents = (f32, usize);

pub fn get_public_keys(participants: &Vec<organization_cert::OrgCert>) -> Vec<String> {
    return participants
        .iter()
        .map(|oc| oc.client_pubkey.clone())
        .collect();
}

/// BEWARE, this function assumes that an organization uses its own OrgCert
pub fn get_index_org_with_pubkey(organizations: &Vec<OrganizationIdentity>, pk: &str) -> usize {
    let orgs: Vec<usize> = organizations
        .iter()
        .enumerate()
        .filter(|(_,o)| o.identity.id_info.org_cert.org_pubkey == pk)
        .map(|(i,_)| i)
        .collect();
    if orgs.len() > 1{
        panic!("Found more than one organization with this public key");
    } else if orgs.len() == 0{
        panic!("Found no organizations with this public key");
    } else {
        return orgs[0];
    }
}


#[test]
pub fn test_reliability_map() {
    // create a default Identity
    let mut id: Identity<u8> = Identity {
        channel_client: 4,
        id_info: IdInfo {
            did_key: [0; 32],
            reliability: 1.0,
            org_cert: organization_cert::OrgCert {
                client_pubkey: String::from(""),
                duration: 0,
                org_pubkey: String::from(""),
                signature: Vec::new()
            }
        },
        reliability_map: HashMap::new(),
        reliability_threshold: 0.5,
        default_reliability: 0.5
    };

    id.update_reliability(vec![(String::from("a"), 0.5)]);
    let a_score = id.calculate_score(id.reliability_map.get("a").unwrap());
    assert_eq!(a_score, 0.5);

    id.update_reliability(vec![(String::from("a"), 1.0)]);
    let a_score = id.calculate_score(id.reliability_map.get("a").unwrap());
    assert_eq!(a_score, 0.75);
}