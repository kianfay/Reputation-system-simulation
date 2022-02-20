use crate::witness_rep::{
    iota_did::create_and_upload_did::Key,
};

use trust_score_generator::trust_score_generators::{
    data_types::messages::signatures::organization_cert,
};

use iota_streams::{
    app::transport::tangle::client::Client,
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
    pub channel_client: C,
    pub id_info: IdInfo,
    pub reliability_map: ReliabilityMap
}

impl<C> Identity<C> {
    pub fn update_reliability(&mut self, new_estimates: Vec<(String, f32)>){
        for (pubkey, estimate) in new_estimates {
            // inserts the default value if the entry does not yet exist
            self.reliability_map.entry(pubkey.clone()).or_insert((0.0, 0));

            // remove the old mapping, and then update and re-insert
            let (score, estimate_count) = self.reliability_map.remove(&pubkey).unwrap();
            self.reliability_map.insert(pubkey, (score + estimate, estimate_count + 1));
        }        
    }
}

pub type ParticipantIdentity = Identity<Subscriber<Client>>;
pub type OrganizationIdentity = Identity<Author<Client>>;

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

pub fn calculate_score(components: &ReliabilityComponents) -> f32 {
    return components.0 / components.1 as f32;
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
        reliability_map: HashMap::new()
    };

    id.update_reliability(vec![(String::from("a"), 0.5)]);
    let a_score = calculate_score(id.reliability_map.get("a").unwrap());
    assert_eq!(a_score, 0.5);

    id.update_reliability(vec![(String::from("a"), 1.0)]);
    let a_score = calculate_score(id.reliability_map.get("a").unwrap());
    assert_eq!(a_score, 0.75);
}