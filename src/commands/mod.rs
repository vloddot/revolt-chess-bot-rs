use reywen::structures::channels::message::Message;
use rust_embed::RustEmbed;

use crate::{Client, ResourceType};

mod moderation;
mod chess;
mod help;

const PREFIX: &str = "!";

#[derive(RustEmbed)]
#[folder = "command-help"]
struct CommandHelp;

#[derive(Debug)]
pub enum Error {
    Generic(String),
    Unimplemented,
    Fetch {
        resource: ResourceType,
        inner: crate::Error,
    },
    InvalidUsage {
        message: String,
        usage: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generic(error) => f.write_str(error),
            Self::Fetch { resource, inner } => {
                write!(f, "Failed to fetch {resource:?}: {inner:?}")
            }
            Self::Unimplemented => {
                write!(
                    f,
                    "This command is not implemented yet, ask the bot owner for more info."
                )
            }
            Self::InvalidUsage { message, usage } => write!(f, "{message}\nUsage:\n>{usage}"),
        }
    }
}

#[async_trait::async_trait]
trait Command {
    fn get_name(&self) -> String;

    fn get_usage(&self) -> String;

    fn get_aliases(&self) -> Vec<String> {
        Vec::new()
    }

    async fn execute(&self, _: &Client, _: &Message) -> Result<(), Error> {
        Err(Error::Unimplemented)
    }
}

pub async fn handle_command(client: &Client, message: &Message) -> Result<(), Error> {
    let Some(content) = &message.content else {
        return Ok(());
    };

    'outer: for command in COMMANDS {
        if content.starts_with(&(PREFIX.to_string() + &command.get_name())) {
            command.execute(client, message).await?;
            break;
        }

        for alias in command.get_aliases() {
            if content.starts_with(&(PREFIX.to_string() + &alias)) {
                command.execute(client, message).await?;
                break 'outer;
            }
        }
    }

    Ok(())
}

pub fn get_help_file(command_name: &str) -> Option<String> {
    let file = CommandHelp::get(&format!("{command_name}.md"))?;

    match String::from_utf8(file.data.to_vec()) {
        Ok(result) => Some(result),
        Err(error) => {
            dbg!(&format!(
                "Failed to retreive short description file {command_name}.md: {error}"
            ));

            None
        }
    }
}

const COMMANDS: &[&(dyn Command + Send + Sync)] = &[&chess::Chess, &help::Help, &moderation::ban::Ban, &moderation::kick::Kick, &moderation::unban::Unban];
