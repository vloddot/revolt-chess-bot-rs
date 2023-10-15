use reywen::{client::methods::message::DataMessageSend, structures::channels::message::Message};
use reywen_http::results::DeltaError;

use crate::{commands::Command, Client};

use super::{Error, ModerationCommand, ModerationCommandInfo};

pub struct Ban;

impl ModerationCommand for Ban {}

#[async_trait::async_trait]
impl Command for Ban {
    fn get_name(&self) -> String {
        "ban".to_string()
    }

    fn get_usage(&self) -> String {
        "<user> [reason]".to_string()
    }

    fn get_aliases(&self) -> Vec<String> {
        vec!["b".to_string()]
    }

    async fn execute(&self, client: &Client, message: &Message) -> Result<(), Error> {
        let ModerationCommandInfo {
            server,
            user_id,
            user_arg,
            reason,
        } = match __self.parse_moderation_command(client, message).await? {
            Some(info) => info,
            None => return Ok(()),
        };

        match client
            .driver
            .ban_create(&server, &user_id, reason.as_deref())
            .await
        {
            Ok(_) => {
                let _ = client
                    .driver
                    .message_send(
                        &message.channel,
                        &DataMessageSend::new().set_content(&format!(
                            "Successfully banned {user_arg} for {}.",
                            reason
                                .map_or(String::from("no reason"), |reason| format!("`{reason}`"))
                        )),
                    )
                    .await;
            }
            Err(error) => {
                return Err(Error::Generic(format!(
                    "Failed to ban {user_arg}: {}",
                    match error {
                        DeltaError::Http(status, body) => match status.as_u16() {
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
