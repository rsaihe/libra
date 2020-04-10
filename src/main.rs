use log::{debug, error, info};
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::StandardFramework;
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::collections::HashSet;
use std::process;
use std::sync::Arc;
use std::time::Instant;

mod commands;
mod utils;

use commands::{FUN_GROUP, GENERAL_GROUP, HELP, OWNER_GROUP};

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

struct PermissionsContainer;

impl TypeMapKey for PermissionsContainer {
    type Value = Permissions;
}

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct StartTime;

impl TypeMapKey for StartTime {
    type Value = Instant;
}

fn main() {
    // Load environment variables from .env file.
    if let Err(e) = kankyo::load(false) {
        eprintln!("Problem loading .env file: {}", e);
    }

    // Start logger.
    env_logger::builder().format_module_path(false).init();

    // Get token from environment.
    let token = match kankyo::key("DISCORD_TOKEN") {
        Some(token) => token,
        None => {
            error!("Expected a token in the environment");
            process::exit(1);
        }
    };

    // Log in with token.
    let mut client = match Client::new(&token, Handler) {
        Ok(client) => client,
        Err(e) => {
            error!("Problem logging in: {}", e);
            process::exit(1);
        }
    };

    // Allow data to be shared across shards.
    {
        let mut data = client.data.write();
        data.insert::<PermissionsContainer>(
            match kankyo::key("PERMS").and_then(|p| p.parse().ok()) {
                Some(p) => Permissions::from_bits_truncate(p),
                None => Permissions::empty(),
            },
        );
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
        data.insert::<StartTime>(Instant::now());
    }

    // Get owner.
    let owners = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);
            set
        }
        Err(e) => {
            error!("Problem accessing application info: {}", e);
            process::exit(1);
        }
    };

    // Configure command framework.
    client.with_framework(
        StandardFramework::new()
            .configure(|c| {
                c.owners(owners).prefix(&match kankyo::key("PREFIX") {
                    Some(prefix) => prefix,
                    None => {
                        error!("Expected a bot prefix in the environment");
                        process::exit(1);
                    }
                })
            })
            .group(&FUN_GROUP)
            .group(&GENERAL_GROUP)
            .group(&OWNER_GROUP)
            .help(&HELP)
            .after(|_, _, command, result| {
                if let Err(e) = result {
                    debug!("Problem in {} command: {:?}", command, e);
                }
            }),
    );

    if let Err(e) = client.start() {
        error!("Client error: {}", e);
        process::exit(1);
    };
}
