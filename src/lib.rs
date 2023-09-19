#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::style,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod commands;
mod util;

use reywen_http::results::DeltaError;
use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use reywen::{
    client::methods::user::DataEditUser,
    structures::{
        channels::Channel,
        media::emoji::Emoji,
        server::Server,
        users::{User, UserStatus},
    },
    websocket::data::{WebSocketEvent, WebSocketSend},
};

#[derive(Debug, Clone, Default)]
pub struct Cache {
    users: HashMap<String, User>,
    servers: HashMap<String, Server>,
    channels: HashMap<String, Channel>,
    emojis: HashMap<String, Emoji>,
    user: Option<User>,
}

#[derive(Clone, Default)]
pub struct Client {
    driver: reywen::client::Client,
    cache: Cache,
}

impl Client {
    ///
    ///
    /// # Panics
    ///
    /// This function will panic if the token could not be initialized.
    #[must_use]
    pub fn from_token(token: &str, is_bot: bool) -> Self {
        Self {
            #[allow(clippy::expect_used)]
            driver: reywen::client::Client::from_token(token, is_bot)
                .expect("Could not initialize client"),
            ..Default::default()
        }
    }

    pub async fn run(&mut self) {
        let (mut read, _) = self.driver.websocket.dual_async().await;
        while let Some(event) = read.next().await {
            if let WebSocketEvent::Ready {
                users,
                servers,
                channels,
                emojis,
            } = event
            {
                self.cache = Cache {
                    users: users
                        .iter()
                        .map(|user| (user.id.clone(), user.clone()))
                        .collect(),

                    servers: servers
                        .iter()
                        .map(|server| (server.id.clone(), server.clone()))
                        .collect(),

                    channels: channels
                        .iter()
                        .map(|channel| (channel.id(), channel.clone()))
                        .collect(),

                    emojis: emojis
                        .iter()
                        .map(|emoji| (emoji.id.clone(), emoji.clone()))
                        .collect(),

                    user: self.fetch_user("@me").await.ok(),
                };
                let _ = self.update_status().await;
                break;
            }
        }

        loop {
            let (mut read, write) = self.driver.websocket.dual_async().await;

            while let Some(event) = read.next().await {
                let this = self.clone();
                let write = write.clone();

                tokio::spawn(async move {
                    match event {
                        WebSocketEvent::Ready { .. } => {
                            let _ = this.update_status().await;
                            let _ = write.lock().await.send(WebSocketSend::ping(0).into()).await;
                        }
                        WebSocketEvent::Pong { data, .. } => {
                            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                            let _ = write
                                .lock()
                                .await
                                .send(WebSocketSend::ping(data).into())
                                .await;
                        }
                        WebSocketEvent::Message { message } => {
                            commands::handle_command(&this, &message);
                        }
                        _ => {}
                    }
                });
            }
        }
    }

    async fn update_status(&self) -> Result<(), DeltaError> {
        let user = match &self.cache.user {
            Some(user) => user.clone(),
            None => self.fetch_user("@me").await?,
        };

        let _ = self
            .driver
            .user_edit(
                &user.id,
                &DataEditUser::new().set_status(
                    UserStatus::new().set_text(&format!("servers: {}", self.cache.servers.len())),
                ),
            )
            .await;

        Ok(())
    }

    async fn fetch_user(&self, id: &str) -> Result<User, DeltaError> {
        match self.cache.users.get(id) {
            Some(user) => Ok(user.clone()),
            None => match self.driver.user_fetch(id).await {
                Ok(user) => Ok(user),
                Err(error) => {
                    dbg!(&format!("Failed to fetch user with ID {id}."));

                    Err(error)
                }
            },
        }
    }
}
