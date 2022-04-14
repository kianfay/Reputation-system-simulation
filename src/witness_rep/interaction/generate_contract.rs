use crate::witness_rep::{
    interaction::user_and_organization::{
        UserIdentity, IdInfo
    },
};

use trust_score_generator::trust_score_generators::data_types::{
    event_protocol_messages::{
        application_messages::exchange_app_messages::CompensationMsg,
        event_protocol_messages::{
            Contract, ApplicationMsg
        },
        contracts::{
            exchange_app_contract::ExchangeContract,
            utility_types::{
                UserOrWitnesses, ParticipantUsers, CompensationJson
            }
        }
    }
};

use iota_streams::{
    core::Result
};
use identity::{
    did::MethodData,
    crypto::KeyPair
};

pub fn generate_exchange_contract(participant_ids: &mut Vec<UserIdentity>) -> Result<Contract> {
    // get the did pubkeys from the ids
    let did_pubkeys_res : Result<Vec<String>> = participant_ids
        .iter()
        .map(|p| {
            let kp = KeyPair::try_from_ed25519_bytes(&p.id_info.did_key)?;
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
    
    let compensation_json: CompensationJson =  vec![
        (UserOrWitnesses::User(String::from("u1")), 0.1)
    ];

    // generate the contract
    let contract_hardcoded = ExchangeContract {
        offer: String::from("tn_b allows tn_a take their place in the queue"),               
        participants: ParticipantUsers(
            did_pubkeys
        ),
        compensation: compensation_json,
        time: 1643572739,
        location: ((53, 20, 27.036),(6, 15, 2.695)),
    };

    return Ok(Contract::ExchangeContract(contract_hardcoded));
}