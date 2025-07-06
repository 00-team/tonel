use std::sync::Arc;

use config::Config;
use db::{Karbar, Settings};
use error::AppErr;
use sqlx::SqlitePool;
use state::{KeyData, State, Store};
use teloxide::adaptors::Throttle;
use teloxide::dispatching::dialogue::ErasedStorage;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;

mod config;
mod db;
mod error;
mod logger;
mod state;
mod utils;

type HR = Result<(), AppErr>;
type TB = Throttle<Bot>;

#[derive(Debug, Clone)]
pub struct Ctx {
    pub db: SqlitePool,
    pub settings: Arc<Mutex<Settings>>,
}

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
    let settings = Arc::new(Mutex::new(Settings::get(&db).await));
    let ctx = Ctx { settings, db };

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, ErasedStorage<State>, State>()
                .branch(
                    dptree::entry()
                        .filter_command::<TonelCommand>()
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
        .dependencies(dptree::deps![storage, ctx])
        .build()
        .dispatch()
        .await;

    Ok(())
}

#[derive(Debug, teloxide::macros::BotCommands, Clone)]
#[command(rename_rule = "snake_case")]
/// Tonel Bot Commands
pub enum TonelCommand {
    Start { r: String },
    Menu,
    Help,
}

pub async fn handle_commands(
    bot: TB, store: Store, ctx: Ctx, msg: Message, cmd: TonelCommand,
) -> HR {
    let Some(user) = msg.from else { return Ok(()) };

    match cmd {
        TonelCommand::Start { r } => {
            let karbar = Karbar::init(&ctx, &user, &r).await?;
            send_menu(&bot, &store, &karbar).await?;
        }
        TonelCommand::Menu => {
            let karbar = Karbar::init(&ctx, &user, "").await?;
            send_menu(&bot, &store, &karbar).await?;
        }
        TonelCommand::Help => {
            let desc = TonelCommand::descriptions().to_string();
            bot.send_message(user.id, desc).await?;
        }
    }

    Ok(())
}

pub async fn send_menu(bot: &TB, store: &Store, karbar: &Karbar) -> HR {
    let menu_text = format!(
        r#"username: {:?}
points: {}
updated_at: {}
name: {}
    "#,
        karbar.username, karbar.points, karbar.updated_at, karbar.fullname
    );

    bot.send_message(karbar.cid(), menu_text)
        .reply_markup(InlineKeyboardMarkup::new([
            vec![InlineKeyboardButton::callback(
                "get daily points",
                KeyData::GetDailyPoints,
            )],
            vec![
                InlineKeyboardButton::callback("get proxy", KeyData::GetProxy),
                InlineKeyboardButton::callback("get v2ray", KeyData::GetV2ray),
            ],
            vec![InlineKeyboardButton::callback(
                "my invite links",
                KeyData::MyInviteLinks,
            )],
        ]))
        .await?;

    store.update(State::Menu).await?;

    Ok(())
}

pub async fn callback_query(
    bot: TB, store: Store, ctx: Ctx, q: CallbackQuery,
) -> HR {
    bot.answer_callback_query(q.id.clone()).await?;
    let Some(data) = &q.data else { return Ok(()) };
    let Some(msg) = q.regular_message() else { return Ok(()) };
    let user = &q.from;
    let settings = Settings::get(&ctx.db).await;
    let conf = Config::get();
    let key = KeyData::from(q.data.clone().unwrap());
    let mut karbar = Karbar::init(&ctx, user, "").await?;
    let state = store.get_or_default().await?;
    let menu_btn = InlineKeyboardButton::callback("main menu", KeyData::Menu);
    let is_admin = conf.admins.contains(&user.id);
    let now = utils::now();

    match key {
        KeyData::Menu => {
            send_menu(&bot, &store, &karbar).await?;
            store.update(State::Menu).await?;
        }
        KeyData::GetDailyPoints => {
            if karbar.last_daily_point_at + Config::DAILY_POINTS_DELAY > now {
                bot.send_message(user.id, "you must wait a day")
                    .reply_markup(InlineKeyboardMarkup::new([[menu_btn]]))
                    .await?;
                return Ok(());
            }

            karbar.points += settings.daily_points;
            karbar.last_daily_point_at = now;
            karbar.set(&ctx).await?;
            send_menu(&bot, &store, &karbar).await?;
        }
        _ => {}
    }

    Ok(())
}

pub async fn handle_messages(
    bot: TB, store: Store, ctx: Ctx, msg: Message,
) -> HR {
    log::info!("msg: {:?}", msg.text());

    Ok(())
}
