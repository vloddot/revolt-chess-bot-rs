use reywen::structures::channels::{message::Message, Channel};

use crate::{Client, ResourceType};

use super::{Command, Error};

pub mod ban;
pub mod kick;
pub mod unban;

struct ModerationCommandInfo {
    server: String,
    user_id: String,
    user_arg: String,
    reason: Option<String>,
}

#[async_trait::async_trait]
trait ModerationCommand: Command {
    async fn parse_moderation_command(
        &self,
        client: &Client,
        message: &Message,
    ) -> Result<Option<ModerationCommandInfo>, Error> {
        let Some(content) = &message.content else {
            return Ok(None);
        };

        let mut args = content.split_whitespace();

        if args.next().is_none() {
            return Ok(None);
        }

        let Some(user_arg) = args.next() else {
            return Err(Error::InvalidUsage {
                message: String::from("User argument needed"),
                usage: self.get_usage(),
            });
        };

        let user = match client.resolve_user(user_arg).await {
            Ok(Some(u)) => u,
            Ok(None) => return Err(Error::Generic(String::from("Could not resolve user"))),
            Err(error) => {
                return Err(Error::Fetch {
                    resource: ResourceType::User,
                    inner: error,
                })
            }
        };

        let reason = args.collect::<Vec<_>>().join(" ");
        let reason = if reason.is_empty() {
            None
        } else {
            Some(reason.as_str())
        };

        let channel = match client.fetch_channel(&message.channel).await {
            Ok(channel) => channel,
            Err(error) => {
                return Err(Error::Fetch {
                    resource: ResourceType::Channel,
                    inner: error,
                });
            }
        };

        let (Channel::TextChannel { server, .. } | Channel::Group { id: server, .. }) = channel
        else {
            return Err(Error::Generic(String::from(
                "Cannot ban outside of text channels and groups",
            )));
        };

        Ok(Some(ModerationCommandInfo {
            server,
            user_id: user.id,
            user_arg: user_arg.to_string(),
            reason: reason.map(String::from),
        }))
    }
}
