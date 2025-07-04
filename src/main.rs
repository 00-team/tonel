use config::Config;
use error::AppErr;
use teloxide::prelude::Requester;

mod config;
mod error;
mod logger;
mod state;

#[tokio::main]
async fn main() -> Result<(), AppErr> {
    log::set_logger(&logger::MasterLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Info);

    log::info!("Start 🐧!");

    let conf = Config::get();
    let bot = Config::init_bot();
    bot.send_message(conf.dev, "Starting Tonel 🌩").await?;

    let storage = Config::init_storage().await;

    Ok(())
}
