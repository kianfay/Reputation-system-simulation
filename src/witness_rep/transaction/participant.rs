use crate::witness_rep::{
    iota_did::create_and_upload_did::Key,
};

use trust_score_generator::trust_score_generators::{
    data_types::messages::signatures::organization_cert,
};

use iota_streams::{
    app::transport::tangle::client::Client,
    app_channels::api::tangle::Subscriber
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

pub type ParticipantIdentity = Identity<Subscriber<Client>>;

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

pub fn calculate_score(components: ReliabilityComponents) -> f32 {
    return components.0 / components.1 as f32;
}


