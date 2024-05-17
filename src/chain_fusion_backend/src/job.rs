use ethers_core::types::U256;
use ic_cdk::api::management_canister::main::raw_rand;
use rand::{
    distributions::{Distribution, WeightedIndex},
    SeedableRng,
};
use rand_chacha::ChaCha20Rng;
use serde_json::{json, to_vec};
use svg::{
    node::element::{Circle, Rectangle},
    Document,
};

use crate::{
    evm_rpc::LogEntry,
    state::{mutate_state, LogSource},
    storage::{store_asset, Asset},
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MintEvent {
    pub token_id: U256,
}

pub struct Attributes {
    pub bg_color: String,
    pub circle_color: String,
}

pub async fn job(event_source: LogSource, event: LogEntry) {
    mutate_state(|s| s.record_processed_log(event_source.clone()));
    let mint_event = MintEvent::from(event);
    let random_bytes = get_random_bytes().await;
    let mut rng = ChaCha20Rng::from_seed(random_bytes);
    let attributes = generate_attributes(&mut rng);
    generate_and_store_metadata(&mint_event, &attributes);
    generate_and_store_image(&mint_event, &attributes);
}

fn generate_and_store_image(mint_event: &MintEvent, attributes: &Attributes) {
    let bg = Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", attributes.bg_color.clone());

    let circle = Circle::new()
        .set("cx", 50)
        .set("cy", 50)
        .set("r", 48)
        .set("fill", attributes.circle_color.clone());

    let document = Document::new()
        .set("viewBox", (0, 0, 100, 100))
        .add(bg)
        .add(circle);

    let body = document.to_string().into_bytes();

    store_asset(
        format!("/{}.svg", mint_event.token_id),
        Asset {
            headers: vec![("Content-Type".to_string(), "image/svg+xml".to_string())],
            body,
        },
    )
}

fn generate_and_store_metadata(mint_event: &MintEvent, attributes: &Attributes) {
    let metadata = json!({
        "name" : format!("dappcon #{}", mint_event.token_id),
        "image" : format!("http://{}.localhost:4943/{}.svg", ic_cdk::id().to_string(), mint_event.token_id),
        "attributes" : [
            {
                "trait_type" : "bg_color",
                "value" : attributes.bg_color
            },
            {
                "trait_type" : "circle_color",
                "value" : attributes.circle_color
            }
        ]
    });

    let body = to_vec(&metadata).expect("json should be serializable");

    store_asset(
        format!("/{}", mint_event.token_id),
        Asset {
            headers: vec![("Content-Type".to_string(), "text/json".to_string())],
            body,
        },
    )
}

fn generate_attributes(rng: &mut ChaCha20Rng) -> Attributes {
    Attributes {
        bg_color: select_value(rng),
        circle_color: select_value(rng),
    }
}

fn select_value(rng: &mut ChaCha20Rng) -> String {
    let svg_color_keywords = ["blue", "red", "green", "yellow"];
    let weights = [3, 3, 3, 1];
    let dist = WeightedIndex::new(weights).unwrap();
    svg_color_keywords[dist.sample(rng)].to_string()
}

async fn get_random_bytes() -> [u8; 32] {
    let (raw_rand,) = raw_rand().await.expect("call should not fail");

    raw_rand.try_into().expect("raw rand should be 32 bytes")
}

impl From<LogEntry> for MintEvent {
    fn from(entry: LogEntry) -> MintEvent {
        // we expect exactly 2 topics from the NewJob event.
        // you can read more about event signatures [here](https://docs.alchemy.com/docs/deep-dive-into-eth_getlogs#what-are-event-signatures)
        let token_id =
            U256::from_str_radix(&entry.topics[3], 16).expect("the token id should be valid");

        MintEvent { token_id }
    }
}
