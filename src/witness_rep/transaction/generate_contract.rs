use crate::witness_rep::{
    transaction::transaction::{ParticipantIdentity,IdInfo},
};

use trust_score_generator::trust_score_generators::data_types::{
    messages::contract::{Contract, TransactingClients}
};

use iota_streams::{
    core::Result
};
use identity::{
    did::MethodData,
    crypto::KeyPair
};

pub fn generate_contract(transacting_ids: &mut Vec<ParticipantIdentity>) -> Result<Contract> {
    // get the did pubkeys from the ids
    let did_pubkeys_res : Result<Vec<String>> = transacting_ids
        .iter()
        .map(|ParticipantIdentity {
            channel_client: _,
            id_info: IdInfo{
                did_key,
                reliability: _,
                org_cert: _
            }
        }| {
            let kp = KeyPair::try_from_ed25519_bytes(did_key)?;
            let multibase_pub = MethodData::new_multibase(kp.public());

            if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
                return Ok(mbpub);
            }
            else {
                return Ok(String::default());
            }
        })
        .collect();
    let did_pubkeys = did_pubkeys_res?;
    
    // generate the contract
    let contract_hardcoded = Contract {
        contract_definition: String::from("tn_b allows tn_a take their place in the queue"),               
        participants: TransactingClients(
            did_pubkeys
        ),      
        time: 1643572739,
        location: ((53, 20, 27.036),(6, 15, 2.695)),
    };

    return Ok(contract_hardcoded);
}