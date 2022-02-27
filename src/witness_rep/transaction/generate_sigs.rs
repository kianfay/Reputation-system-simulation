use trust_score_generator::trust_score_generators::{
    data_types::{
        messages::{
            contract::{Contract},
            signatures::{
                witness_sig, transacting_sig, organization_cert
            },
            tx_messages::WitnessClients
        }
    },
};

use iota_streams::core::Result;
use identity::{
    did::MethodData,
    crypto::{KeyPair, Ed25519, Sign}
};

use ed25519_dalek::{Sha512, Digest};



pub fn generate_witness_sig(
    contract: Contract,
    channel_pk_as_multibase: String,
    did_keypair: KeyPair,
    org_cert: organization_cert::OrgCert,
    timeout: u32,
    hash_len: usize
) -> Result<witness_sig::WitnessSig> {

    // get the public key in multibase encoding
    let did_pk_as_multibase: String = get_multibase(&did_keypair);

    // serialize, then hash and truncate the contract
    let ser_con = serde_json::to_string(&contract)?;
    let hash = Sha512::digest(ser_con.as_bytes());
    let trun_hash = Vec::from(&hash[0..hash_len]);

    // WN signs their response
    let wn_pre_sig = witness_sig::WitnessPreSig {
        contract: trun_hash.clone(),
        signer_channel_pubkey: channel_pk_as_multibase.clone(),
        org_cert: org_cert.clone(),
        timeout: timeout,
    };
    let wn_pre_sig_bytes = serde_json::to_string(&wn_pre_sig)?;
    let wn_sig_bytes: [u8; 64]  = Ed25519::sign(&String::into_bytes(wn_pre_sig_bytes), did_keypair.private())?;

    // WN packs the signature bytes in with the signiture message
    let wn_sig = witness_sig::WitnessSig {
        contract: trun_hash,
        signer_channel_pubkey: channel_pk_as_multibase,
        org_cert: org_cert,
        timeout: timeout,
        signer_did_pubkey: did_pk_as_multibase,
        signature: wn_sig_bytes.to_vec(),
    };

    return Ok(wn_sig);
}

pub fn generate_transacting_sig(
    contract: Contract,
    channel_pk_as_multibase: String,
    did_keypair: KeyPair,
    witnesses: WitnessClients,
    witness_sigs: transacting_sig::ArrayOfWnSignituresBytes,
    org_cert: organization_cert::OrgCert,
    timeout: u32,
    hash_len: usize
) -> Result<transacting_sig::TransactingSig> {
    
    // get the public key in multibase encoding
    let did_pk_as_multibase: String = get_multibase(&did_keypair);

    // serialize, then hash and truncate the contract
    let ser_con = serde_json::to_string(&contract)?;
    let hash_con = Sha512::digest(ser_con.as_bytes());
    let trun_hash_con = Vec::from(&hash_con[0..hash_len]);

    // serialize, then hash and truncate the witness node sigs
    witnesses.sort();
    let ser_wns = serde_json::to_string(&witness_sigs)?;
    let hash_wns = Sha512::digest(ser_wns.as_bytes());
    let trun_hash_wns = Vec::from(&hash_wns[0..hash_len]);

    // TN_A signs the transaction
    let tn_a_tx_msg_pre_sig = transacting_sig::TransactingPreSig {
        contract: trun_hash_con.clone(),
        signer_channel_pubkey: channel_pk_as_multibase.clone(),
        wit_node_sigs: trun_hash_wns.clone(),
        org_cert: org_cert.clone(),
        timeout: timeout
    };
    let tn_a_tx_msg_pre_sig_bytes = serde_json::to_string(&tn_a_tx_msg_pre_sig)?;
    let tn_a_tx_msg_sig: [u8; 64]  = Ed25519::sign(&String::into_bytes(tn_a_tx_msg_pre_sig_bytes), did_keypair.private())?;

    // TN_A packs the signature bytes in with the signiture message
    let tn_a_sig = transacting_sig::TransactingSig {
        contract: trun_hash_con,
        signer_channel_pubkey: channel_pk_as_multibase.clone(),
        wit_node_sigs: trun_hash_wns,
        org_cert: org_cert,
        timeout: timeout,
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
) -> Result<organization_cert::OrgCert>{
    let pre_sig = organization_cert::OrgCertPreSig {
        client_pubkey: client_pubkey.clone(),
        duration: timeout
    };
    let pre_sig_bytes = serde_json::to_string(&pre_sig)?;
    let pre_sig_bytes: [u8; 64]  = Ed25519::sign(&String::into_bytes(pre_sig_bytes), org_did_keypair.private())?;

    let did_pk_as_multibase: String = get_multibase(org_did_keypair);

    let org_cert = organization_cert::OrgCert {
        client_pubkey,
        duration: timeout,
        org_pubkey: did_pk_as_multibase,
        signature: pre_sig_bytes.to_vec()
    };
    return Ok(org_cert);
}

pub fn get_multibase(did_keypair: &KeyPair) -> String {
    let multibase_pub = MethodData::new_multibase(did_keypair.public());
    if let MethodData::PublicKeyMultibase(mbpub) = multibase_pub {
        return mbpub;
    }
    else {
        panic!("Could not encode public key as multibase")
    }
}