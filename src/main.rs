extern crate config;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};
use state::Storage;
use std::fs;
use std::{collections::HashMap, error::Error};

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

struct Configuration {
    channel_id: String,
    discord_token: String,
    aitum_api: String,
    aitum_show_rule: String,
    aitum_hide_rule: String,
    aitum_show_rule_id: String,
    aitum_hide_rule_id: String,
}
static CONFIG: Storage<RwLock<Configuration>> = Storage::new();

#[derive(Serialize, Deserialize)]
struct AtiumRules {
    status: String,
    data: HashMap<String, String>,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        // let channel_id = CONFIG.get().channel_id.parse::<u64>().unwrap();
        let config = CONFIG.get().read().await;
        let channel_id = match config.channel_id.parse::<u64>() {
            Ok(val) => val,
            Err(_e) => return,
        };

        if msg.channel_id.as_u64() != &channel_id {
            return;
        }

        fs::write("currentsong.txt", &msg.content).expect("Unable to write file");
        println!("Updated song: {}", msg.content);

        if !config.aitum_hide_rule_id.is_empty() && !config.aitum_show_rule.is_empty() {
            let mut rule_id = &config.aitum_show_rule_id;
            if msg.content == "-" {
                rule_id = &config.aitum_hide_rule_id;
            }

            let aitum_result =
                reqwest::get(config.aitum_api.to_owned() + "/aitum/rules/" + rule_id).await;

            match aitum_result {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to ping Aitum: {}", e);
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = config::Config::default();
    settings
        // Add in `./config.toml`
        .merge(config::File::with_name("config.toml"))
        .unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("APP"))
        .unwrap();

    let mut configuration = Configuration {
        discord_token: settings
            .get_str("DISCORD_TOKEN")
            .expect("DISCORD_TOKEN is required in config.toml"),
        channel_id: settings
            .get_str("CHANNEL_ID")
            .expect("CHANNEL_ID is required in config.toml"),
        aitum_show_rule: settings
            .get_str("AITUM_SHOW_RULE")
            .expect("AITUM_SHOW_RULE is required in config.toml"),
        aitum_hide_rule: settings
            .get_str("AITUM_HIDE_RULE")
            .expect("AITUM_HIDE_RULE is required in config.toml"),
        aitum_api: "".to_string(),
        aitum_show_rule_id: "".to_string(),
        aitum_hide_rule_id: "".to_string(),
    };

    println!("Searching for Atium master instance...");
    configuration.aitum_api = find_aitum_master()
        .await
        .expect("Unable to find Atium master instance");

    // Find the Aitum rule ids
    let resp = reqwest::get(configuration.aitum_api.to_owned() + "/aitum/rules")
        .await?
        .json::<AtiumRules>()
        .await?;

    configuration.aitum_show_rule_id = resp
        .data
        .get(&configuration.aitum_show_rule)
        .unwrap_or_else(|| {
            panic!(
                "Could not find rule in Aitum: {}",
                configuration.aitum_show_rule
            )
        })
        .to_string();

    configuration.aitum_hide_rule_id = resp
        .data
        .get(&configuration.aitum_hide_rule)
        .unwrap_or_else(|| {
            panic!(
                "Could not find rule in Aitum: {}",
                configuration.aitum_hide_rule
            )
        })
        .to_string();

    CONFIG.set(RwLock::new(configuration));

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let token = &CONFIG.get().read().await.discord_token;
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}

async fn find_aitum_master() -> Option<String> {
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let service_type = "_pebble._tcp.local.";
    let receiver = mdns.browse(service_type).expect("Failed to browse");

    let mut result: Option<String> = None;
    while let Ok(event) = receiver.recv_async().await {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                result = Some(format!("http://{}:7777", info.get_hostname()));
                println!("Found Atium master instance: {}", info.get_hostname());
                break;
            }
            _ => {}
        }
    }
    let _ = mdns.shutdown();

    return result;
}
