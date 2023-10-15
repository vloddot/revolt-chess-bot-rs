use chess::{ChessMove, Color, Piece, Square};
use futures_util::{SinkExt, StreamExt};
use regex::Regex;
use reywen::{
    client::methods::message::DataMessageSend,
    structures::channels::message::{Message, Reply},
    websocket::data::{WebSocketEvent, WebSocketSend},
};

use super::{Command, Error, PREFIX};
use crate::{Client, ResourceType};

#[derive(Default)]
pub struct Chess;

#[async_trait::async_trait]
impl Command for Chess {
    fn get_name(&self) -> String {
        "chess".to_string()
    }

    fn get_aliases(&self) -> Vec<String> {
        vec!["play-chess".to_string()]
    }

    fn get_usage(&self) -> String {
        "[white|black|random] <opponent>".to_string()
    }

    async fn execute(&self, client: &Client, message: &Message) -> Result<(), super::Error> {
        let Some(content) = &message.content else {
            return Ok(());
        };

        let mut args = content.split_whitespace();

        // the command argument
        if args.next().is_none() {
            return Ok(());
        }

        let Some(p1_color) = args.next() else {
            return Err(Error::InvalidUsage {
                message: String::from("Color argument needed."),
                usage: self.get_usage(),
            });
        };

        let Some(p1_color) = get_color(p1_color) else {
            return Err(Error::InvalidUsage {
                message: format!("Unexpected color \"{p1_color}\"."),
                usage: self.get_usage(),
            });
        };

        let Some(p2) = args.next() else {
            return Err(Error::InvalidUsage {
                message: String::from("Opponent argument needed"),
                usage: self.get_usage(),
            });
        };

        let p2 = match client.resolve_user(p2).await {
            Ok(Some(p2)) => p2,
            Ok(None) => return Err(Error::Generic(String::from("Failed to find user."))),
            Err(error) => return Err(Error::Fetch {
                resource: ResourceType::User,
                inner: error
            }),
        };

        let p1 = match client.fetch_user(&message.author).await {
            Ok(p) => p,
            Err(error) => {
                return Err(Error::Fetch {
                    resource: ResourceType::User,
                    inner: error,
                });
            }
        };

        let mut game = chess::Game::new();
        let mut current_turn_is_p1 = matches!(p1_color, Color::White);

        loop {
            let (mut read, write) = client.driver.websocket.dual_async().await;

            while let Some(event) = read.next().await {
                match event {
                    WebSocketEvent::Ready { .. } => {
                        let _ = write
                            .lock()
                            .await
                            .send(WebSocketSend::Ping { data: 0 }.into())
                            .await;
                    }
                    WebSocketEvent::Pong { data } => {
                        let write = write.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                            let _ = write
                                .lock()
                                .await
                                .send(WebSocketSend::Ping { data }.into())
                                .await;
                        });
                    }
                    WebSocketEvent::Message { message } => {
                        let Some(content) = message.content else {
                            continue;
                        };

                        let user_id = if current_turn_is_p1 { &p1.id } else { &p2.id };
                        if message.author.as_str() != user_id {
                            continue;
                        }

                        let mut args = content.split_whitespace();

                        if !args
                            .next()
                            .is_some_and(|command| *command == format!("{PREFIX}move"))
                        {
                            continue;
                        }

                        let replies = vec![Reply {
                            id: message.id,
                            mention: true,
                        }];

                        let Some(uci_move) = args.next() else {
                            let _ = client
                                .driver
                                .message_send(
                                    &message.channel,
                                    &DataMessageSend::new().set_content(&format!("Expected UCI move argument. Usage:\n> {PREFIX}move [a-h][1-8][a-h][1-8](r|n|b|q)?")).set_replies(replies),
                                )
                                .await;
                            continue;
                        };

                        let Some(captures) = Regex::new("^([a-h][1-8])([a-h][1-8])(r|n|b|q)?$")
                            .unwrap()
                            .captures(uci_move)
                        else {
                            println!("invalid move");
                            continue;
                        };

                        let Some(start_square) = captures.get(1) else {
                            println!("no captures[1]");
                            continue;
                        };

                        let Some(target_square) = captures.get(2) else {
                            println!("no captures[2]");
                            continue;
                        };

                        let promotion = captures.get(3).map(|promotion| match promotion.as_str() {
                            "r" => Piece::Rook,
                            "n" => Piece::Knight,
                            "b" => Piece::Bishop,
                            "q" => Piece::Queen,
                            _ => unreachable!(),
                        });

                        let mut start_square = start_square.as_str().chars();
                        let mut target_square = target_square.as_str().chars();

                        let start_file = start_square.next().unwrap() as u8 - b'a';
                        let start_rank = start_square.next().unwrap() as u8 - b'0';
                        let target_file = target_square.next().unwrap() as u8 - b'a';
                        let target_rank = target_square.next().unwrap() as u8 - b'0';

                        let (start_square, target_square) = unsafe {
                            (
                                Square::new(start_file + start_rank * 8),
                                Square::new(target_file + target_rank * 8),
                            )
                        };

                        println!("{:?} {:?}", start_square, target_square);
                        game.make_move(ChessMove::new(start_square, target_square, promotion));

                        println!("{:?}", game.current_position());
                        current_turn_is_p1 = !current_turn_is_p1;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn get_color(color: &str) -> Option<Color> {
    match color {
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        "random" => Some(if rand::random::<f32>() > 0.5 {
            Color::White
        } else {
            Color::Black
        }),
        _ => None,
    }
}
