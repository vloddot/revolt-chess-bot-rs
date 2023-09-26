#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let client = revolt_chess_bot_rs::Client::from_token(
        &std::env::var("BOT_TOKEN")
            .expect(
                "Could not receive variable `BOT_TOKEN` from environment variables, did you forget to set it in the `.env` file?"
            ),
        true
    ).await;
    client.run().await;
}
