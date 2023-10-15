use reywen::{client::methods::message::DataMessageSend, structures::channels::message::Message};
use reywen_http::results::DeltaError;

use crate::{commands::Command, Client};

use super::{Error, ModerationCommand, ModerationCommandInfo};

pub struct Kick;

impl ModerationCommand for Kick {}

#[async_trait::async_trait]
impl Command for Kick {
    fn get_name(&self) -> String {
        "kick".to_string()
    }

    fn get_usage(&self) -> String {
        "<user> [reason]".to_string()
    }

    fn get_aliases(&self) -> Vec<String> {
        vec!["k".to_string()]
    }

    async fn execute(&self, client: &Client, message: &Message) -> Result<(), Error> {
        let ModerationCommandInfo {
            server,
            user_id,
            user_arg,
            reason,
        } = match self.parse_moderation_command(client, message).await? {
            Some(info) => info,
            None => return Ok(()),
        };

        match client.driver.member_kick(&server, &user_id).await {
            Ok(_) => {
                let _ = client
                    .driver
                    .message_send(
                        &message.channel,
                        &DataMessageSend::new().set_content(&format!(
                            "Successfully kicked {user_arg} for {}.",
                            reason
                                .map_or(String::from("no reason"), |reason| format!("`{reason}`"))
                        )),
                    )
                    .await;
            }
            Err(error) => {
                return Err(Error::Generic(format!(
                    "Failed to kick {user_arg}: {}",
                    match error {
                        DeltaError::Http(status, body) => match status.as_u16() {
                            404 => "User not in server.".to_string(),
                            400 => "Invalid operation.".to_string(),
                            code => format!("Status code {code}: `{body}`"),
                        },
                        error => format!("`{error:?}`"),
                    }
                )));
            }
        };

        Ok(())
    }
}
