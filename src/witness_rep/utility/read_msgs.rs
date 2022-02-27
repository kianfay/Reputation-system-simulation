use iota_streams::{
    app_channels::api::tangle::UnwrappedMessage,
    app::transport::tangle::client::Client,
    app_channels::api::tangle::{
        Address, Subscriber
    },
    core::Result,
};

use core::str::FromStr;

pub async fn read_msgs(
    node_url: &str, 
    ann_msg: &str, 
    seed: &str
) -> Result<Vec<UnwrappedMessage>> {
    // build another client to read the tangle with (the seed must be the same as 
    // one of the subscribers in the channel, which is why the author's seed is needed)
    let client = Client::new_from_url(node_url);
    let mut reader = Subscriber::new(seed, client.clone());

    // process the address string
    let ann_address = Address::from_str(ann_msg)?;
    reader.receive_announcement(&ann_address).await?;

    // fetch messages from address, and extract their payloads
    return reader.fetch_next_msgs().await;
}