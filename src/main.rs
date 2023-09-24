#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    revolt_chess_bot_rs::CLIENT.write().await.set_token(&std::env::var("BOT_TOKEN").expect("Could not receive variable `BOT_TOKEN` from environment variables, did you forget to set it in the `.env` file?"), true);
    revolt_chess_bot_rs::run_client().await;
}
