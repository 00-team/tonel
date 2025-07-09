use book::Book;
use config::Config;
use db::{Karbar, Proxy, Settings};
use error::AppErr;
use sqlx::SqlitePool;
use state::{KeyData, State, Store};
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::Arc;
use teloxide::adaptors::Throttle;
use teloxide::dispatching::dialogue::ErasedStorage;
use teloxide::error_handlers::ErrorHandler;
use teloxide::net::Download;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;

mod book;
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

impl<E: Debug> ErrorHandler<E> for SendDevErrorHandler {
    fn handle_error(self: Arc<Self>, error: E) -> BoxFuture<'static, ()> {
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

    let mut keyboard = vec![
        vec![
            InlineKeyboardButton::callback(
                "get daily points",
                KeyData::GetDailyPoints,
            ),
            InlineKeyboardButton::callback("free config", KeyData::FreeVpn),
        ],
        vec![
            InlineKeyboardButton::callback("get proxy", KeyData::GetProxy),
            InlineKeyboardButton::callback("get v2ray", KeyData::GetV2ray),
        ],
        vec![InlineKeyboardButton::callback(
            "my invite links",
            KeyData::MyInviteLinks,
        )],
    ];

    if karbar.is_admin() {
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                "forced join",
                KeyData::AdminForceJoinList,
            ),
            InlineKeyboardButton::callback("send all", KeyData::AdminSendAll),
        ]);
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                "proxy list",
                KeyData::AdminProxyList,
            ),
            InlineKeyboardButton::callback(
                "v2ray list",
                KeyData::AdminV2rayList,
            ),
            InlineKeyboardButton::callback(
                "set free config",
                KeyData::AdminSetFreeVpn,
            ),
        ]);
    }

    bot.send_message(karbar.cid(), menu_text)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
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
    // let conf = Config::get();
    let key = KeyData::from(data);
    let mut karbar = Karbar::init(&ctx, user, "").await?;
    let state = store.get_or_default().await?;
    let is_admin = karbar.is_admin();
    let now = utils::now();

    let cid = msg.chat.id;
    let mid = msg.id;

    if is_admin {
        match state {
            State::Menu => match key {
                KeyData::AdminProxyList => {
                    admin_proxy_list(&bot, &store, &ctx, 0, user.id).await?;
                }
                _ => {}
            },
            State::AdminProxyList => match key {
                KeyData::BookAdd => {
                    bot.delete_message(cid, mid).await?;
                    bot.send_message(
                        user.id,
                        concat!(
                            "send a proxy links. each link must be on ",
                            "a different line. like:\n\nproxy 1\nproxy 2\n",
                            "proxy 3.\n\n send in a message or a .txt file"
                        ),
                    )
                    .reply_markup(KeyData::main_menu())
                    .await?;
                    store.update(State::AdminProxyAdd).await?;
                }
                KeyData::BookItem(page, id) => {
                    let px = Proxy::get(&ctx, id).await?;
                    bot.delete_message(cid, mid).await?;
                    bot.send_message(
                        user.id,
                        format!(
                            "Proxy:\n\nserver: {}\nport: {}\n\nsecret: {}",
                            px.server, px.port, px.secret
                        ),
                    )
                    .reply_markup(InlineKeyboardMarkup::new([
                        [
                            InlineKeyboardButton::callback(
                                if px.disabled { "enable" } else { "disable" },
                                KeyData::AdminProxyDisabledToggle(page, px.id),
                            ),
                            InlineKeyboardButton::callback(
                                "delete ⭕",
                                KeyData::AdminProxyDel(page, px.id),
                            ),
                        ],
                        [
                            InlineKeyboardButton::callback(
                                "<- back",
                                KeyData::BookPagination(page),
                            ),
                            KeyData::main_menu_btn(),
                        ],
                    ]))
                    .await?;
                }
                KeyData::BookPagination(page) => {
                    bot.delete_message(cid, mid).await?;
                    admin_proxy_list(&bot, &store, &ctx, page, user.id).await?;
                }
                _ => {}
            },
            _ => {}
        }

        match key {
            KeyData::AdminProxyDel(page, id) => {
                Proxy::del(&ctx, id).await?;
                bot.delete_message(cid, mid).await?;
                admin_proxy_list(&bot, &store, &ctx, page, user.id).await?;
            }
            KeyData::AdminProxyDisabledToggle(page, id) => {
                Proxy::disabled_toggle(&ctx, id).await?;
                bot.delete_message(cid, mid).await?;
                admin_proxy_list(&bot, &store, &ctx, page, user.id).await?;
            }
            _ => {}
        }
    }

    match key {
        KeyData::Menu => {
            send_menu(&bot, &store, &karbar).await?;
            store.update(State::Menu).await?;
        }
        KeyData::Nothing => {
            bot.delete_message(msg.chat.id, msg.id).await?;
        }
        KeyData::GetDailyPoints => {
            if karbar.last_daily_point_at + Config::DAILY_POINTS_DELAY > now {
                bot.send_message(user.id, "you must wait a day")
                    .reply_markup(KeyData::main_menu())
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
    let Some(user) = &msg.from else { return Ok(()) };
    let karbar = Karbar::init(&ctx, user, "").await?;
    let state = store.get_or_default().await?;
    let is_admin = karbar.is_admin();

    if is_admin {
        match state {
            State::AdminProxyAdd => {
                admin_proxy_add(&bot, &msg, &ctx).await?;
            }
            _ => {}
        }
    }

    log::info!("msg: {:?}", msg.text());

    Ok(())
}

async fn admin_proxy_add(bot: &TB, msg: &Message, ctx: &Ctx) -> HR {
    let cid = msg.chat.id;
    let mut data = msg.text().map(|v| v.to_string()).unwrap_or_default();

    'd: {
        let Some(doc) = msg.document() else { break 'd };
        if doc.file.size > 2 * 1024 * 1024 {
            bot.send_message(cid, "max file size is 2MB").await?;
            break 'd;
        }
        let m = doc.mime_type.clone();

        if !m.map(|v| v.type_() == "text").unwrap_or_default() {
            bot.send_message(cid, "only text files are allowed").await?;
            break 'd;
        }

        let f = bot.get_file(doc.file.id.clone()).await?;
        let mut buf = Vec::with_capacity(f.size as usize);
        bot.download_file(&f.path, &mut buf).await?;
        match String::from_utf8(buf.clone()) {
            Ok(v) => data += &v,
            Err(e) => {
                let nb = buf[..e.utf8_error().valid_up_to()].to_vec();
                if let Ok(d) = String::from_utf8(nb) {
                    data += &d;
                }
            }
        }
    };

    let mut added = 0;

    for line in data.split('\n') {
        if line.is_empty() {
            continue;
        }

        let Some(mut px) = Proxy::from_link(line) else { continue };
        if px.add(ctx).await.is_ok() {
            added += 1;
        }
    }

    bot.send_message(
        cid,
        format!(
            "added {added} new proxies\n\nsend other proxies or go to menu"
        ),
    )
    .reply_markup(KeyData::main_menu())
    .await?;

    Ok(())
}

async fn admin_proxy_list(
    bot: &TB, store: &Store, ctx: &Ctx, page: u32, uid: UserId,
) -> HR {
    let proxies = Proxy::list(&ctx, page).await?;
    let count = Proxy::count(&ctx).await?;
    let bk = Book::new(proxies, page, count / 32);

    bot.send_message(
        uid,
        format!("Proxy List Page {page}\n\n{}", &bk.message()),
    )
    .parse_mode(ParseMode::Html)
    .reply_markup(bk.keyboard())
    .await?;
    store.update(State::AdminProxyList).await?;

    Ok(())
}
