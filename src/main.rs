mod achievement_list;
mod resume;

use anyhow::Context as _;
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::all::{
    GuildId, CreateCommand, CreateCommandOption, Interaction, 
    CreateInteractionResponseMessage, CreateInteractionResponse
};
use shuttle_runtime::SecretStore;
use tracing::info;

struct Bot {
    discord_guild_id: GuildId,
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    
        let commands = vec![
            CreateCommand::new("hello").description("Say hello"),
            CreateCommand::new("resume").description("Display the résumé")
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "character_id",
                        "loadstone character id",
                    )
                    .required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::String,
                        "resume_type",
                        "all/u: Ultimate/s: Savage",
                    )
                    .required(false),
                ),
        ];
    
        let commands = &self
            .discord_guild_id
            .set_commands(&ctx.http, commands)
            .await
            .unwrap();
    
        info!("Registered commands: {:#?}", commands);
    }
    // WIP
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = interaction {
            let response_content = match command.data.name.as_str() {
                "hello" => "hello".to_owned(),
                "resume" => {
                    let character_id_option = command
                        .data
                        .options
                        .iter()
                        .find(|opt| opt.name == "character_id")
                        .cloned();

                    let resume_type_option = command
                        .data
                        .options
                        .iter()
                        .find(|opt| opt.name == "resume_type")
                        .cloned();
                    
                    if let Some(character_id_value) = character_id_option {
                        let character_id = character_id_value.value.as_str().unwrap();

                        // let resume_type = resume_type_option.and_then(|opt| opt.value.as_str()).unwrap_or("all");
                        let resume_type = resume_type_option
                            .and_then(|opt| opt.value.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| "all".to_string());

                        let result = resume::get_resume(character_id, &resume_type).await;
                        match result {
                            Ok((character_information, character_resume)) => {
                                format!("résumé: {}\n{}", character_information, character_resume)
                            }
                            Err(err) => {
                                format!("Err: {}", err)
                            }
                        }
                    } else {
                        "Missing required option: place".to_owned()
                    }
                }
                command => unreachable!("Unknown command: {}", command),
            };

            
            let data = CreateInteractionResponseMessage::new().content(response_content.clone());
            let builder = CreateInteractionResponse::Message(data);
            if let Err(why) = command.create_response(&ctx.http, builder).await {
                println!("Cannot respond to slash command: {why}");
            }
        }
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secret_store: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    let discord_token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let discord_guild_id = secret_store
        .get("DISCORD_GUILD_ID")
        .context("'DISCORD_GUILD_ID' was not found")?;

    let client = get_client(
        &discord_token,
        discord_guild_id.parse().unwrap(),
    )
    .await;

    Ok(client.into())
}

pub async fn get_client(
    discord_token: &str,
    discord_guild_id: u64,
) -> Client {
    let intents = GatewayIntents::empty();

    Client::builder(discord_token, intents)
        .event_handler(
            Bot {
                discord_guild_id: GuildId::new(discord_guild_id),
            }
        )
        .await
        .expect("Err creating client")
}