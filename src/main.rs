use config::Config;
use db::{Karbar, Settings};
use error::{AppErr, Worm};
use session::Session;
use sqlx::SqlitePool;
use state::{State, Store};
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::Arc;
use teloxide::adaptors::Throttle;
use teloxide::dispatching::dialogue::ErasedStorage;
use teloxide::error_handlers::ErrorHandler;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
// use tokio::sync::Mutex;

mod book;
mod cbq;
mod config;
mod db;
mod error;
mod logger;
mod payam;
mod session;
mod state;
mod utils;

type HR = Result<(), AppErr>;
type TB = Throttle<Bot>;

#[derive(Debug, Clone)]
pub struct Ctx {
    pub db: SqlitePool,
    // pub settings: Arc<Mutex<Settings>>,
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
    // let settings = Arc::new(Mutex::new(Settings::get(&db).await));
    let ctx = Ctx { db };

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .enter_dialogue::<Message, ErasedStorage<State>, State>()
                .branch(
                    dptree::entry()
                        .filter_command::<TonelCommand>()
                        .endpoint(handle_commands),
                )
                .endpoint(payam::Payam::handle),
        )
        .branch(Update::filter_pre_checkout_query().endpoint(handle_pcq))
        .branch(
            Update::filter_callback_query()
                .enter_dialogue::<Update, ErasedStorage<State>, State>()
                .endpoint(cbq::Cbq::handle),
        );

    let eh = SendDevErrorHandler { bot: bot.clone(), dev: conf.dev };

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![storage, ctx])
        .error_handler(Arc::new(eh))
        .build()
        .dispatch()
        .await;

    Ok(())
}

struct SendDevErrorHandler {
    bot: TB,
    dev: UserId,
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

impl ErrorHandler<AppErr> for SendDevErrorHandler {
    fn handle_error(self: Arc<Self>, error: AppErr) -> BoxFuture<'static, ()> {
        if matches!(
            error.worm,
            Worm::Blocked | Worm::Banned | Worm::MessageToDeleteNotFound
        ) {
            return Box::pin(async {});
        }

        let msg = format!("error: {error:#?}");
        let bot = self.bot.clone();
        Box::pin(async move {
            if let Err(e) = bot.send_message(self.dev, msg).await {
                log::error!("error sending to dev: {e:#?}");
            }
        })
    }
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
            let mut it = r.splitn(2, '-');
            let key = it.next().unwrap_or_default();
            let val = it.next().unwrap_or_default();
            let code = if key == "inv" { val } else { "" };
            let karbar = Karbar::init(&ctx, &user, code).await?;
            let mut s = Session {
                cid: msg.chat.id,
                settings: Settings::get(&ctx.db).await,
                ctx,
                now: utils::now(),
                karbar,
                conf: Config::get(),
                bot,
                store,
            };

            s.ch_send().await?;
            s.send_welcome().await?;

            match key {
                "donate" => s.donate().await?,
                _ => s.send_menu().await?,
            };
        }
        TonelCommand::Menu => {
            let karbar = Karbar::init(&ctx, &user, "").await?;
            let mut s = Session {
                cid: msg.chat.id,
                settings: Settings::get(&ctx.db).await,
                ctx,
                now: utils::now(),
                karbar,
                conf: Config::get(),
                bot,
                store,
            };
            s.ch_send().await?;
            s.send_menu().await?;
        }
        TonelCommand::Help => {
            let desc = TonelCommand::descriptions().to_string();
            bot.send_message(user.id, desc).await?;
        }
    }

    Ok(())
}

pub async fn handle_pcq(bot: TB, ctx: Ctx, q: PreCheckoutQuery) -> HR {
    Karbar::init(&ctx, &q.from, "").await?;
    // store.update(State::Menu).await?;
    bot.answer_pre_checkout_query(q.id, true).await?;
    Ok(())
}
