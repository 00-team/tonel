use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, OnceLock},
    time::Duration,
};
use teloxide::{
    Bot, dispatching::dialogue::ErasedStorage, net::default_reqwest_settings,
    prelude::RequesterExt, types::UserId,
};

use crate::state::State;

mod config_toml {
    use std::path::PathBuf;

    #[derive(Debug, serde::Deserialize)]
    pub struct Bot {
        pub token: String,
        pub admins: Vec<u64>,
        pub dev: u64,
        pub storage: String,
        pub username: String,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct Db {
        pub path: String,
    }

    #[derive(Debug, serde::Deserialize)]
    pub struct ConfigToml {
        pub bot: Bot,
        pub db: Db,
    }

    fn path() -> PathBuf {
        let mut args = std::env::args();
        let path = loop {
            let Some(arg) = args.next() else { break None };
            if arg == "-c" || arg == "--config" {
                break args.next();
            }
        }
        .unwrap_or(String::from("config.toml"));

        PathBuf::from(path)
    }

    pub fn get() -> ConfigToml {
        let path = path();
        let data = match std::fs::read_to_string(&path) {
            Ok(v) => v,
            Err(e) => panic!("could not read config at: {path:?}\n{e:#?}"),
        };

        match toml::from_str(&data) {
            Ok(v) => v,
            Err(e) => panic!("invalid toml config file: {path:?}\n{e:#?}"),
        }
    }
}

#[derive(Debug)]
/// Tonel Config
pub struct Config {
    bot_token: String,
    bot_storage: String,
    db_path: String,
    pub admins: HashSet<UserId>,
    pub dev: UserId,
    pub start_url: reqwest::Url,
    pub donate_url: reqwest::Url,
    /// without @
    pub bot_username: String,
}

impl Config {
    /// 24 hours
    pub const DAILY_POINTS_DELAY: i64 = 24 * 3600;
    /// 24 hours
    pub const PRICE_STACK_RESET: i64 = 24 * 3600;
    pub const CODE_ABC: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    pub const SEND_ALL_SLEEP: Duration = Duration::from_secs(10);

    fn init() -> Self {
        let ct = config_toml::get();
        let su = format!("https://t.me/{}?start=x", ct.bot.username);
        let start_url = reqwest::Url::from_str(&su).expect("bad start url");
        let du = format!("https://t.me/{}?start=donate", ct.bot.username);
        let donate_url = reqwest::Url::from_str(&du).expect("bad donate url");

        Self {
            bot_token: ct.bot.token,
            bot_storage: ct.bot.storage,
            db_path: ct.db.path,
            admins: ct.bot.admins.iter().map(|id| UserId(*id)).collect(),
            dev: UserId(ct.bot.dev),
            bot_username: ct.bot.username,
            start_url,
            donate_url,
        }
    }

    pub fn get() -> &'static Self {
        static STATE: OnceLock<Config> = OnceLock::new();
        STATE.get_or_init(Self::init)
    }

    pub fn init_bot() -> teloxide::adaptors::Throttle<Bot> {
        let config = Self::get();
        let builder = default_reqwest_settings();
        let client = if cfg!(debug_assertions) {
            let p = reqwest::Proxy::all("socks5h://127.0.0.1:9898").unwrap();
            builder.proxy(p)
        } else {
            builder
        }
        .build()
        .expect("could not build the bot client");

        Bot::with_client(&config.bot_token, client)
            .throttle(teloxide::adaptors::throttle::Limits::default())
    }

    pub async fn init_storage() -> Arc<ErasedStorage<State>> {
        use teloxide::dispatching::dialogue::serializer::Json;
        use teloxide::dispatching::dialogue::{SqliteStorage, Storage};

        let conf = Self::get();

        SqliteStorage::open(&conf.bot_storage, Json)
            .await
            .expect("could not init teloxide sqlite state storage")
            .erase()
    }

    pub async fn init_db() -> SqlitePool {
        let conf = Self::get();
        let uri = format!("sqlite://{}", conf.db_path);
        let cpt = SqliteConnectOptions::from_str(&uri)
            .expect(&format!(
                "could not init sqlite connection with uri: {uri}"
            ))
            .journal_mode(SqliteJournalMode::Off);

        SqlitePool::connect_with(cpt)
            .await
            .expect(&format!("sqlite connection failed with: {uri}"))
    }
}
