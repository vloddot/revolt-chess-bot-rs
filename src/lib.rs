#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::style,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod commands;

use once_cell::sync::Lazy;
use reywen_http::results::DeltaError;
use std::collections::HashMap;
use tokio::sync::RwLock;

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

pub static CLIENT: Lazy<RwLock<Client>> = Lazy::new(|| RwLock::new(Client::default()));

pub async fn run_client() {
    let (mut read, _) = CLIENT.read().await.driver.websocket.dual_async().await;
    while let Some(event) = read.next().await {
        if let WebSocketEvent::Ready {
            users,
            servers,
            channels,
            emojis,
        } = event
        {
            let user = CLIENT.write().await.fetch_user("@me").await.ok();
            CLIENT.write().await.cache = Cache {
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
                user,
            };
            let _ = CLIENT.write().await.update_status().await;
            break;
        }
    }

    loop {
        let (mut read, write) = CLIENT.read().await.driver.websocket.dual_async().await;

        while let Some(event) = read.next().await {
            let write = write.clone();

            tokio::spawn(async move {
                match event {
                    WebSocketEvent::Ready { .. } => {
                        let _ = CLIENT.write().await.update_status().await;
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
                        commands::handle_command(message).await;
                    }
                    _ => {}
                }
            });
        }
    }
}

impl Client {
    /// Sets the session token for the `Client`.
    ///
    /// # Panics
    ///
    /// This function will panic if the client could not be initialized.
    #[allow(clippy::expect_used)]
    pub fn set_token(&mut self, token: &str, is_bot: bool) {
        self.driver =
            reywen::client::Client::from_token(token, is_bot).expect("Could not initialize client");
    }

    async fn update_status(&mut self) -> Result<(), DeltaError> {
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

    async fn fetch_user(&mut self, id: &str) -> Result<User, DeltaError> {
        match self.cache.users.get(id) {
            Some(user) => Ok(user.clone()),
            None => match self.driver.user_fetch(id).await {
                Ok(user) => {
                    self.cache.users.insert(user.id.clone(), user.clone());

                    Ok(user)
                }
                Err(error) => {
                    dbg!(&format!("Failed to fetch user with ID {id}."));

                    Err(error)
                }
            },
        }
    }
}
