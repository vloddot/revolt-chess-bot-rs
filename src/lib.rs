#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::style,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod commands;

use redis::{Commands, RedisError};
use reywen_http::results::DeltaError;

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

#[derive(Clone)]
pub struct Client {
    driver: reywen::client::Client,
    user: Option<User>,
    db: redis::Client,
}

pub enum Error {
    Delta(DeltaError),
    Redis(RedisError),
}

impl From<DeltaError> for Error {
    fn from(value: DeltaError) -> Self {
        Self::Delta(value)
    }
}

impl From<RedisError> for Error {
    fn from(value: RedisError) -> Self {
        Self::Redis(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! redis_json_wrapper {
    ($name:ident, $inner:ident) => {
        #[derive(serde::Deserialize, serde::Serialize)]
        pub struct $name($inner);

        impl redis::ToRedisArgs for $name {
            fn write_redis_args<W>(&self, out: &mut W)
            where
                W: ?Sized + redis::RedisWrite,
            {
                out.write_arg(&serde_json::to_vec(self).unwrap());
            }
        }

        impl redis::FromRedisValue for $name {
            fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
                if let redis::Value::Data(data) = v {
                    Ok(serde_json::from_slice(&data).unwrap())
                } else {
                    panic!("invalid JSON received: {v:?}")
                }
            }
        }
    };
}

redis_json_wrapper!(RedisUser, User);
redis_json_wrapper!(RedisServer, Server);
redis_json_wrapper!(RedisChannel, Channel);
redis_json_wrapper!(RedisEmoji, Emoji);

impl Client {
    pub async fn run(&self) {
        'try_connection: loop {
            let (mut read, _) = self.driver.websocket.dual_async().await;
            while let Some(event) = read.next().await {
                if let WebSocketEvent::Ready {
                    users,
                    servers,
                    channels,
                    emojis,
                } = event
                {
                    let mut conn = self.db.get_connection().unwrap();
                    let _: redis::RedisResult<()> = conn.hset_multiple(
                        "users",
                        &users
                            .iter()
                            .map(|user| (&user.id, RedisUser(user.clone())))
                            .collect::<Vec<(_, _)>>(),
                    );
                    let _: redis::RedisResult<()> = conn.hset_multiple(
                        "servers",
                        &servers
                            .iter()
                            .map(|server| (&server.id, RedisServer(server.clone())))
                            .collect::<Vec<(_, _)>>(),
                    );
                    let _: redis::RedisResult<()> = conn.hset_multiple(
                        "channels",
                        &channels
                            .iter()
                            .map(|channel| (channel.id(), RedisChannel(channel.clone())))
                            .collect::<Vec<(_, _)>>(),
                    );
                    let _: redis::RedisResult<()> = conn.hset_multiple(
                        "emojis",
                        &emojis
                            .iter()
                            .map(|emoji| (&emoji.id, RedisEmoji(emoji.clone())))
                            .collect::<Vec<(_, _)>>(),
                    );
                    drop(conn);
                    let _ = self.update_status().await;
                    break 'try_connection;
                }
            }
        }

        loop {
            let (mut read, write) = self.driver.websocket.dual_async().await;

            while let Some(event) = read.next().await {
                let write = write.clone();

                match event {
                    WebSocketEvent::Ready { .. } => {
                        let _ = self.update_status().await;
                        let _ = write.lock().await.send(WebSocketSend::ping(0).into()).await;
                    }
                    WebSocketEvent::Pong { data, .. } => {
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                            let _ = write
                                .lock()
                                .await
                                .send(WebSocketSend::ping(data).into())
                                .await;
                        });
                    }
                    WebSocketEvent::Message { message } => {
                        let this = self.clone();
                        tokio::spawn(async move {
                            commands::handle_command(this, message).await;
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    pub async fn from_token(token: &str, is_bot: bool) -> Self {
        let mut this = Self {
            driver: reywen::client::Client::from_token(token, is_bot)
                .expect("Failed to initialize client"),
            user: None,
            db: redis::Client::open("redis://127.0.0.1/").expect("Failed to connect to Redis DB"),
        };

        this.user = this.fetch_user("@me").await.ok();
        this
    }

    async fn update_status(&self) -> Result<()> {
        let user = match &self.user {
            Some(user) => user.clone(),
            None => self.fetch_user("@me").await?,
        };

        let mut conn = self.db.get_connection().unwrap();
        let server_count: usize = conn.hlen("servers")?;

        self.driver
            .user_edit(
                &user.id,
                &DataEditUser::new()
                    .set_status(UserStatus::new().set_text(&format!("servers: {server_count}"))),
            )
            .await?;

        Ok(())
    }

    async fn fetch_user(&self, id: &str) -> std::result::Result<User, DeltaError> {
        let user = self.driver.user_fetch(id).await?;

        let conn = self.db.get_connection();
        if let Ok(mut conn) = conn {
            let _: redis::RedisResult<()> = conn.hset("users", id, RedisUser(user.clone()));
        }

        Ok(user)
    }
}
