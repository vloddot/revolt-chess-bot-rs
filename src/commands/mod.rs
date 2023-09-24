use reywen::structures::channels::message::Message;
use rust_embed::RustEmbed;

mod chess;
mod help;

const PREFIX: &str = "!";

#[derive(RustEmbed)]
#[folder = "command-help"]
struct CommandHelp;

#[async_trait::async_trait]
trait Command {
    fn get_name(&self) -> String;

    fn get_aliases(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_usage(&self) -> String;

    async fn execute(&self, _: Message) {
        unimplemented!("Command {} is unimplemented.", self.get_name());
    }
}

pub async fn handle_command(message: Message) {
    let Some(content) = &message.content else {
        return;
    };

    'outer: for command in COMMANDS {
        if content.starts_with(&(PREFIX.to_string() + &command.get_name())) {
            command.execute(message).await;
            break;
        }

        for alias in command.get_aliases() {
            if content.starts_with(&(PREFIX.to_string() + &alias)) {
                command.execute(message).await;
                break 'outer;
            }
        }
    }
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

const COMMANDS: &[&(dyn Command + Send + Sync)] = &[&chess::Chess, &help::Help];
