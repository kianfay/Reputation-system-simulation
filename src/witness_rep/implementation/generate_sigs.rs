use wb_reputation_system::data_types::{
    event_protocol_messages::{
        event_protocol_messages::{
            Contract
        },
        signatures::{
            witness_sig, interaction_sig,
            organization_cert::{
                OrganizationCertificatePreSig, OrganizationCertificate
            },
        },
        application_constructs::application_contracts::{
            utility_types::WitnessUsers
        }
    },
};

use iota_streams::{
    core::{println, Result},
};
use identity::{
    did::MethodData,
    crypto::{KeyPair, Ed25519, Sign}
};

use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_witness_sig(
    contract: Contract,
    channel_pk_as_multibase: String,
    did_keypair: KeyPair,
    org_cert: OrganizationCertificate,
    duration: u32
) -> Result<witness_sig::WitnessSig> {


    let did_pk_as_multibase: String = get_multibase(&did_keypair);

    // WN signs their response
    let wn_pre_sig = witness_sig::WitnessPreSig {
        contract: contract.clone(),
        signer_channel_pubkey: channel_pk_as_multibase.clone(),
        org_cert: org_cert.clone(),
        timeout: get_timeout(duration),
    };
    let wn_pre_sig_bytes = serde_json::to_string(&wn_pre_sig)?;
    let wn_sig_bytes: [u8; 64]  = Ed25519::sign(&String::into_bytes(wn_pre_sig_bytes), did_keypair.private())?;

    // WN packs the signature bytes in with the signiture message
    let wn_sig = witness_sig::WitnessSig {
        contract: contract.clone(),
        signer_channel_pubkey: channel_pk_as_multibase,
        org_cert: org_cert,
        timeout: get_timeout(duration),
        signer_did_pubkey: did_pk_as_multibase,
        signature: wn_sig_bytes.to_vec(),
    };

    return Ok(wn_sig);
}

pub fn generate_participant_sig(
    contract: Contract,
    channel_pk_as_multibase: String,
    did_keypair: KeyPair,
    witnesses: WitnessUsers,
    witness_sigs: interaction_sig::ArrayOfWnSignituresBytes,
    org_cert: OrganizationCertificate,
    duration: u32
) -> Result<interaction_sig::InteractionSig> {

    let did_pk_as_multibase: String = get_multibase(&did_keypair);

    // TN_A signs the interaction
    let tn_a_tx_msg_pre_sig = interaction_sig::InteractionPreSig {
        contract: contract.clone(),
        signer_channel_pubkey: channel_pk_as_multibase.clone(),
        witnesses: witnesses.clone(),
        wit_node_sigs: witness_sigs.clone(),
        org_cert: org_cert.clone(),
        timeout: get_timeout(duration)
    };
    let tn_a_tx_msg_pre_sig_bytes = serde_json::to_string(&tn_a_tx_msg_pre_sig)?;
    let tn_a_tx_msg_sig: [u8; 64]  = Ed25519::sign(&String::into_bytes(tn_a_tx_msg_pre_sig_bytes), did_keypair.private())?;

    // TN_A packs the signature bytes in with the signiture message
    let tn_a_sig = interaction_sig::InteractionSig {
        contract: contract.clone(),
        signer_channel_pubkey: channel_pk_as_multibase.clone(),
        witnesses: witnesses,
        wit_node_sigs: witness_sigs,
        org_cert: org_cert,
        timeout: get_timeout(duration),
        signer_did_pubkey: did_pk_as_multibase.clone(),
        signature: tn_a_tx_msg_sig.to_vec()
    };

    return Ok(tn_a_sig);
}

// generates the organization certificate (needed by participants)
pub fn generate_org_cert(
    client_pubkey: String,
    org_did_keypair: &KeyPair,
    timeout: u32
) -> Result<OrganizationCertificate>{
    let pre_sig = OrganizationCertificatePreSig {
        client_pubkey: client_pubkey.clone(),
        timeout: timeout
    };
    let pre_sig_bytes = serde_json::to_string(&pre_sig)?;
    let pre_sig_bytes: [u8; 64]  = Ed25519::sign(&String::into_bytes(pre_sig_bytes), org_did_keypair.private())?;

    let did_pk_as_multibase: String = get_multibase(org_did_keypair);

    let org_cert = OrganizationCertificate {
        client_pubkey,
        timeout: timeout,
        org_pubkey: did_pk_as_multibase,
        signature: pre_sig_bytes.to_vec()
    };
    return Ok(org_cert);
}

pub fn get_multibase(did_keypair: &KeyPair) -> String {
    let did_pk_as_multibase: String;
    let multibase_pub = MethodData::new_multibase(did_keypair.public());
    if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
        return mbpub;
    }
    else {
        panic!("Could not encode public key as multibase")
    }
}

pub fn get_timeout(duration: u32) -> u32 {
    let start = SystemTime::now();
    let current_time = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    return current_time.as_secs() as u32 + duration;
}