extern crate config;
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
    aitum_enabled: bool,
    aitum_api: String,
    aitum_show_rule: String,
    aitum_hide_rule: String,
    aitum_show_rule_id: String,
    aitum_hide_rule_id: String,
}
static CONFIG: Storage<RwLock<Configuration>> = Storage::new();

#[derive(Serialize, Deserialize)]
struct AitumRules {
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

        if config.aitum_enabled {
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
        aitum_enabled: settings
            .get_bool("AITUM_ENABLED")
            .expect("AITUM_ENABLED is required in config.toml"),
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

    if configuration.aitum_enabled {
        configuration.aitum_api = settings
            .get_str("AITUM_API")
            .expect("AITUM_API is required in config.toml");

        // Find the Aitum rule ids
        let resp = reqwest::get(configuration.aitum_api.to_owned() + "/aitum/rules")
            .await
            .unwrap_or_else(|_| panic!("Failed to communicate with Aitum"))
            .json::<AitumRules>()
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
    }

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
