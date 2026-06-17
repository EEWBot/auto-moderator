use std::fs::File;
use std::io::Read;
use std::time::SystemTime;

use clap::Parser;
use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::futures::StreamExt;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use crate::model::{Cli, Config, Moderation};

mod model;

struct Handler {
    config: Config,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let Some(guild) = msg.guild_id else {
            return;
        };

        let Some(server_config) = self.config.servers.get(&guild.into()) else {
            return;
        };

        if !server_config.traps.contains(&msg.channel_id.into()) {
            return;
        }

        let channel_name = match msg.channel_id.name(&ctx.http).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to get channel name: {e}");
                "UNKNOWN".to_string()
            }
        };

        if server_config.whitelist.contains(&msg.author.id.into()) {
            tracing::info!(
                "{} ({}) was triggerd but whitelisted.",
                msg.author.name,
                msg.author.id
            );

            return;
        }

        let warn_msg = format!(
            "Trap triggered! ({} - {} in {channel_name})",
            msg.author.name, msg.author.id
        );

        let trapped_in = SystemTime::now();

        tracing::warn!("{warn_msg}");

        if let Some(message) = &server_config.moderation_message {
            match msg.author.create_dm_channel(&ctx.http).await {
                Ok(channel) => {
                    if let Err(e) = channel
                        .send_message(&ctx.http, CreateMessage::new().content(message))
                        .await
                    {
                        tracing::warn!(
                            "Failed to send DM channel to {} - {}: {e}",
                            msg.author.name,
                            msg.author.id
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create DM channel to {} - {}: {e}",
                        msg.author.name,
                        msg.author.id
                    );
                }
            }
        }

        match server_config.moderation {
            Moderation::Disabled => tracing::info!("Do nothing"),
            Moderation::Kick => {
                if let Err(e) = guild
                    .kick_with_reason(&ctx.http, msg.author.id, &warn_msg)
                    .await
                {
                    tracing::warn!("Failed to KICK user: {e}, ignored.");
                }
            }
        }

        tokio::time::sleep(server_config.message_removal_delay).await;
        tracing::info!("Start message cleaning");

        let removal_target_timestamp = (trapped_in - server_config.message_removal_range)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let channels = match guild.channels(&ctx.http).await {
            Ok(channels) => channels,
            Err(e) => {
                tracing::warn!("Failed to get channels from guild: {e}");
                return;
            }
        };

        for channel_id in channels.keys() {
            let mut channel_stream = channel_id.messages_iter(&ctx.http).boxed();

            let channel_name = match channel_id.name(&ctx.http).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to get channel name: {e}");
                    "UNKNOWN".to_string()
                }
            };

            while let Some(message_result) = channel_stream.next().await {
                match message_result {
                    Ok(m) => {
                        let message_timestamp = m.timestamp.timestamp() as u64;

                        if message_timestamp < removal_target_timestamp {
                            break;
                        }

                        if m.author.id == msg.author.id {
                            match m.delete(&ctx.http).await {
                                Ok(_) => {}
                                Err(e) => tracing::warn!("Failed to remove message: {e}"),
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to fetch mesasge for channel {channel_name} ({channel_id}): {e}"
                        );
                        break;
                    }
                }
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        tracing::info!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let mut config = String::new();
    let mut config_f = File::open(&cli.config).expect("Failed to open config file");
    config_f
        .read_to_string(&mut config)
        .expect("Failed to read config file");

    let config: Config = toml::from_str(&config).expect("Failed to parse config toml");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&cli.discord_token, intents)
        .event_handler(Handler { config })
        .await
        .expect("Failed to create client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
