use crate::witness_rep::{
    interaction::user_and_organization::{
        UserIdentity
    },
};

use wb_reputation_system::data_types::{
    event_protocol_messages::{
        event_protocol_messages::{
            Contract
        },
        application_constructs::application_contracts::{
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

// informally requires the participant_ids vector to be of length 2
pub fn generate_exchange_contract(
    participant_ids: &mut Vec<UserIdentity>,
    channel_address: String
) -> Result<Contract> {
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
        channel_address: channel_address,
        offer: String::from("p1 allows p2 take their place in the queue"),               
        participants: ParticipantUsers(
            vec![(did_pubkeys[0].clone(), String::from("p1")), (did_pubkeys[1].clone(), String::from("p2"))]
        ),
        compensation: compensation_json,
        
        //metadata
        time: 1643572739,
        location: ((53, 20, 27.036),(6, 15, 2.695)),
        timeout: 1643573739
    };

    return Ok(Contract::ExchangeApplication(contract_hardcoded));
}