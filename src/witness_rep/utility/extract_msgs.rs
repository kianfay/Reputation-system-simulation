use iota_streams::app_channels::api::tangle::{
    MessageContent, UnwrappedMessage
};
use identity::{
    did::MethodData
};

// Ectracts all message payloads and pubkeys from each branch. start_branch
// starts at 0.
pub fn extract_msg(
    retrieved_msgs: Vec<UnwrappedMessage>,
    start_branch: usize
) -> Vec<Vec<(String, String)>> {
    let mut messages: Vec<Vec<(String, String)>> = Vec::new();
    let mut current_branch = 0;

    println!("Length: {}", retrieved_msgs.len());
    retrieved_msgs
        .iter()
        .for_each(|msg| {
            /* println!("whole obj:{:?}", msg);
            println!("just body:{:?}\n", msg.body); */
            let content = &msg.body;
            match content {
                MessageContent::SignedPacket {
                    pk,
                    public_payload,
                    masked_payload: _,
                } => {
                    if current_branch > start_branch{
                        let pay = String::from_utf8(public_payload.0.to_vec()).unwrap();
                        let pubk = MethodData::new_multibase(pk);
                        if let MethodData::PublicKeyMultibase(mbpub) = pubk {
                            messages.last_mut().unwrap().push((pay, mbpub));
                        } else {
                            panic!("Failed to decode public key")
                        }
                    }
                },
                MessageContent::Keyload  => {
                    // create a new vector for each keyload
                    if current_branch > start_branch{
                        let new_vec: Vec<(String, String)> = Vec::new();
                        messages.push(new_vec);
                    }
                    current_branch += 1;
                }
                _ => {
                    panic!("Simulation only built to handle signed and keyload messages");
                }
            }
        });
    
    return messages;
}