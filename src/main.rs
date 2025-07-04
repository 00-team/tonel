use config::Config;
use error::AppErr;
use sqlx::SqlitePool;
use state::{State, Store, TonelCommands};
use teloxide::adaptors::Throttle;
use teloxide::dispatching::dialogue::{ErasedStorage, GetChatId};
use teloxide::prelude::*;

mod config;
mod error;
mod logger;
mod state;

type HR = Result<(), AppErr>;
type TB = Throttle<Bot>;

#[tokio::main]
async fn main() -> Result<(), AppErr> {
    log::set_logger(&logger::MasterLogger).expect("could not init logger");
    log::set_max_level(log::LevelFilter::Info);

    log::info!("Start 🐧!");

    let conf = Config::get();
    let bot = Config::init_bot();
    bot.send_message(conf.dev, "Starting Tonel 🌩").await?;

    let storage = Config::init_storage().await;
    let db = Config::init_db().await;

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, ErasedStorage<State>, State>()
                .branch(
                    dptree::entry()
                        .filter_command::<TonelCommands>()
                        .endpoint(handle_commands),
                )
                .endpoint(handle_messages),
        )
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<Update, ErasedStorage<State>, State>()
                .endpoint(callback_query),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage, db])
        .build()
        .dispatch()
        .await;

    Ok(())
}

pub async fn handle_commands(
    bot: TB, store: Store, db: SqlitePool, msg: Message, cmd: TonelCommands,
) -> HR {
    log::info!("cmd: {cmd:#?}");

    Ok(())
}

pub async fn callback_query(
    bot: TB, store: Store, db: SqlitePool, q: CallbackQuery,
) -> HR {
    bot.answer_callback_query(q.id).await?;
    Ok(())
}

pub async fn handle_messages(
    bot: TB, store: Store, db: SqlitePool, msg: Message,
) -> HR {
    log::info!("msg: {:?}", msg.text());


    Ok(())
}
