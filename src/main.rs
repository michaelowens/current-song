extern crate config;
use state::LocalStorage;
use std::error::Error;
use std::fs;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

struct Configuration {
    channel_id: String,
    discord_token: String
}
static CONFIG: LocalStorage<Configuration> = LocalStorage::new();

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        // let channel_id = CONFIG.get().channel_id.parse::<u64>().unwrap();
        let channel_id = match CONFIG.get().channel_id.parse::<u64>() {
            Ok(val) => val,
            Err(_e) => return
        };

        if msg.channel_id.as_u64() != &channel_id {
            return;
        }

        fs::write("currentsong.txt", &msg.content).expect("Unable to write file");
        println!("Updated song: {}", msg.content);
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
	let mut settings = config::Config::default();
    settings
        // Add in `./config.toml`
        .merge(config::File::with_name("config.toml")).unwrap()
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("APP")).unwrap();


    CONFIG.set(move || Configuration {
        discord_token: settings.get_str("DISCORD_TOKEN").expect("DISCORD_TOKEN is required in config.toml"),
        channel_id: settings.get_str("CHANNEL_ID").expect("CHANNEL_ID is required in config.toml")
    });

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let token = &CONFIG.get().discord_token;
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }

    Ok(())
}