use crate::witness_rep::{
    iota_did::create_and_upload_did::Key,
};

use wb_reputation_system::{
    data_types::{
        event_protocol_messages::signatures::organization_cert::{
            OrganizationCertificate
        },
        identity::identity::Identity
    },
};

use iota_streams::{
    app::transport::tangle::client::Client,
    app_channels::api::tangle::{Subscriber, Author}
};


pub type UserIdentity = Identity<Subscriber<Client>, IdInfo>;
pub struct OrganizationIdentity{
    pub identity:  Identity<Author<Client>, IdInfo>,
    pub ann_msg: Option<String>,
}

// This is all of the external information about a participant, including their
// decentralised ID and their reliability.
pub struct IdInfo {
    pub seed: Option<String>,
    pub did_key: Key,
    pub reliability: Option<f32>,
    pub org_cert: OrganizationCertificate
}



pub fn get_public_keys(participants: &Vec<OrganizationCertificate>) -> Vec<String> {
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


/* #[test]
pub fn test_reputation_map() {
    // create a default Identity
    let mut id: Identity<u8, IdInfo> = Identity {
        channel_client: 4,
        seed: String::from(" "),
        id_info: IdInfo {
            did_key: [0; 32],
            reliability: 1.0,
            org_cert: organization_cert::OrgCert {
                client_pubkey: String::from(""),
                timeout: 0,
                org_pubkey: String::from(""),
                signature: Vec::new()
        },
        reputation_map: HashMap::new(),
        user_reliability_threshold: 0.5,
        user_default_reliability: 0.5
    };

    id.update_reliability(vec![(String::from("a"), 0.5)]);
    let a_score = id.calculate_score(id.reputation_map.get("a").unwrap());
    assert_eq!(a_score, 0.5);

    id.update_reliability(vec![(String::from("a"), 1.0)]);
    let a_score = id.calculate_score(id.reputation_map.get("a").unwrap());
    assert_eq!(a_score, 0.75);
} */