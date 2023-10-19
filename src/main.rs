use std::env;

use config::Config;
use event_handler::Handler;
use serenity::{framework::StandardFramework, prelude::GatewayIntents, Client};
use songbird::{driver::DecodeMode, SerenityInit};

mod audio_receiver;
mod config;
mod event_handler;

async fn create_client(token: &str, id: u64) -> Result<Client, serenity::Error> {
    let framework = StandardFramework::new().configure(|c| c.with_whitespace(true));

    let songbird_config = songbird::Config::default().decode_mode(DecodeMode::Decode);

    Client::builder(token, GatewayIntents::all())
        .event_handler(Handler)
        .application_id(id)
        .framework(framework)
        .register_songbird_from_config(songbird_config)
        .await
}

#[tokio::main]
async fn main() {
    let config = {
        let config = std::fs::read_to_string("./config.toml");
        if let Ok(config) = config {
            toml::from_str::<Config>(&config).expect("Cannot load config file.")
        } else {
            let token = env::var("BOT_TOKEN").unwrap();
            let application_id = env::var("BOT_ID").unwrap();

            Config {
                token,
                application_id: u64::from_str_radix(&application_id, 10).unwrap(),
            }
        }
    };

    let mut client = create_client(&config.token, config.application_id)
        .await
        .expect("Err creating client");

    if let Err(err) = client.start().await {
        println!("Client error: {err:?}");
    }
}
