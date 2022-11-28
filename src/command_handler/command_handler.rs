use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    client::Context,
    model::{
        application::interaction::{
            application_command::ApplicationCommandInteraction, InteractionResponseType,
        },
        id::GuildId,
        prelude::{
            Message,
            interaction::application_command::{CommandDataOption, CommandDataOptionValue},
        },
    },
    framework::{
        standard::CommandResult,
    }
};

use crate::{database_handler, user_info::*, command_handler::{commands, command_return::CommandReturn} , DBContainer, loa_contents::LOA_CONTENTS, embed_pages};

use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;
use log::error;

#[async_trait]
pub trait CommandInterface {
    async fn run(
        &self, 
        ctx: &Context, 
        command: &ApplicationCommandInteraction, 
        options: &[CommandDataOption]
    ) -> CommandReturn;
    fn register<'a: 'b, 'b>(
        &'a self,
        command: &'a mut CreateApplicationCommand
    ) -> &'b mut CreateApplicationCommand;
}

pub struct CommandList {
    pub commands: HashMap<&'static str, Box<dyn CommandInterface + Send + Sync>>,
}

impl CommandList {
    pub async fn register(&'static self, gid: GuildId, ctx: &Context) {
        for (_, command) in &self.commands {
            if let Err(why) = gid
                .create_application_command(&ctx.http, |c| command.register(c))
                .await
            {
                println!("Cannot create application command: {:#?}", why);
            }
        }
    }
}

lazy_static! {
    pub static ref COMMAND_LIST: CommandList = CommandList {
        commands: HashMap::from([
            ("조회", commands::character_query::command()),
            ("사용자초기화", commands::user_reset::command()),
            ("등록", commands::user_register::command()),
        ])
    };
}

pub async fn execute_command(ctx: &Context, command: ApplicationCommandInteraction) {

    command.defer(&ctx.http).await.unwrap();

    let cmd_result = match COMMAND_LIST.commands.get(command.data.name.as_str()) {
        Some(result) => result.run(&ctx, &command, &command.data.options).await,
        None => CommandReturn::String("".to_string()),
    };

    match cmd_result {
        CommandReturn::String(content) => {
            if let Err(why) = command
                .edit_original_interaction_response(&ctx.http, |msg| msg.content(&content))
                .await
            {
                error!(
                    "Failed to send Single-string \"{:?}\" from command \"{}\".",
                    content, command.data.name
                );
                error!("{:#?}", why);
            }
        }
        CommandReturn::SingleEmbed(embed) => {
            if let Err(why) = command
                .edit_original_interaction_response(&ctx.http, |msg| msg.set_embed(embed.clone()))
                .await
            {
                error!(
                    "Failed to send single-embed \"{:#?}\" from command \"{}\".",
                    embed, command.data.name
                );
                error!("{:#?}", why);
            }
        }
        CommandReturn::EmbedPages(pages) => {
            if let Err(why) = command
                .edit_original_interaction_response(&ctx.http, |msg| {
                    msg.set_embed(pages.pages[0].clone())
            })
                .await
            {
                error!(
                    "Failed to send embed pages\"{:#?}\" from command \"{}\".",
                    pages, command.data.name
                );
                error!("{:#?}", why);
            }

            if let Err(why) = embed_pages::control_pages(ctx, command, pages).await {
                error!("an error occured while handling embed pages.");
                error!("{:#?}", why);
            }
        }
        _ => ()
    }

}

pub async fn msg_from_user_info(ctx: &Context, userinfo: &UserInfo) -> String {
    let mut result = String::from(userinfo.user_name());
    for (name, charinfo) in userinfo.user_character().iter() {
        result.push_str(format!("\n닉네임: {}, 클래스: {}, 레벨: {}, 수입: {}", 
            name, 
            charinfo.class(),
            charinfo.lv(),
            LOA_CONTENTS.cal_gold(&charinfo.total_hw())
        ).as_str());
    }
    result
}