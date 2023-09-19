use reywen::{
    client::methods::message::DataMessageSend,
    structures::channels::message::{Message, Reply},
};

use crate::Client;

use super::{get_help_file, Command, COMMANDS};

pub struct Help;

impl Command for Help {
    fn get_name(&self) -> String {
        "help".to_string()
    }

    fn get_aliases(&self) -> Vec<String> {
        vec!["h".to_string()]
    }

    fn get_usage(&self) -> String {
        "[command]".to_string()
    }

    fn execute(&self, client: Client, message: Message) {
        tokio::spawn(async move {
            let Some(content) = &message.content else {
                return;
            };

            let mut args = content.split_ascii_whitespace();

            if args.next().is_none() {
                return;
            }

            let replies = vec![Reply {
                id: message.id,
                mention: true,
            }];

            let content = if let Some(command_name) = args.next() {
                let matches_command = |command: &&&(dyn Command + Send + Sync)| {
                    command_name == command.get_name()
                        || command.get_aliases().contains(&command_name.to_string())
                };

                let Some(command) = COMMANDS.iter().find(matches_command) else {
                    let _ = client
                        .driver
                        .message_send(
                            &message.channel,
                            &DataMessageSend::new()
                                .set_content(&format!(
                                    "Command {} does not exist. Execute /help for a list of commands.",
                                    command_name
                                ))
                                .set_replies(replies),
                        )
                        .await;
                    return;
                };

                let name = command.get_name();
                let aliases = command.get_aliases();

                let mut text = format!("# `{name}` Command Details\n\n");

                text.push_str(&format!(
                    "Usage:\n> !{name} {usage}\n\n",
                    usage = command.get_usage()
                ));

                text.push_str(
                    &get_help_file(&name).map_or_else(String::new, |description| {
                        format!("Description:\n{description}\n\n")
                    }),
                );

                text.push_str(&if aliases.is_empty() {
                    String::new()
                } else {
                    format!(
                        "Aliases: {}",
                        aliases
                            .iter()
                            .map(|alias| format!("`{alias}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                });

                text
            } else {
                format!(
                    "{intro}\n{commands}",
                    intro = include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/command-help/intro.md"
                    )),
                    commands = format!(
                        "Commands:\n{}",
                        COMMANDS
                            .iter()
                            .map(|command| format!("`{}`", command.get_name()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                )
            };

            let _ = client
                .driver
                .message_send(
                    &message.channel,
                    &DataMessageSend::new()
                        .set_content(&content)
                        .set_replies(replies),
                )
                .await;
        });
    }
}
