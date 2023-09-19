use revolt_chess_bot_rs::Client;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let token = std::env::var("BOT_TOKEN").expect("Could not receive variable `BOT_TOKEN` from environment variables, did you forget to set it in the `.env` file?");
    let mut client = Client::from_token(&token, true);
    client.run().await;
}
