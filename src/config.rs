use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};
use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, OnceLock},
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
}

// macro_rules! evar {
//     ($name:literal) => {
//         std::env::var($name).expect(concat!($name, " was not found in env"))
//     };
// }

impl Config {
    // pub fn gooje_url(&self, path: &str) -> String {
    //     format!("{}{path}", self.gooje_host)
    // }

    fn init() -> Self {
        let ct = config_toml::get();

        // use reqwest::header;
        // let mut gooje_headers = header::HeaderMap::new();
        // gooje_headers.insert(
        //     header::AUTHORIZATION,
        //     header::HeaderValue::from_str(&format!(
        //         "golem {}",
        //         et.gooje.golem_auth
        //     ))
        //     .expect("invalid auth header for gooje client"),
        // );

        // let gooje_client = Client::builder()
        //     .default_headers(gooje_headers)
        //     .connect_timeout(std::time::Duration::from_secs(10))
        //     .connection_verbose(false)
        //     .build()
        //     .expect("could not build gooje client");

        Self {
            bot_token: ct.bot.token,
            bot_storage: ct.bot.storage,
            db_path: ct.db.path,
            admins: ct.bot.admins.iter().map(|id| UserId(*id)).collect(),
            dev: UserId(ct.bot.dev),
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
