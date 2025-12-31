use std::fmt::Debug;
use teloxide::{ApiError, DownloadError, RequestError};

#[derive(Debug)]
pub enum Worm {
    Unknown,
    NotFound,
    AlreadyExists,
    Banned,
    Blocked,
    MessageToDeleteNotFound,
    TxRq(RequestError),
    Sqlx(sqlx::Error),
    Down(DownloadError),
    Rqw(reqwest::Error),
}

#[derive(Debug)]
pub struct AppErr {
    pub worm: Worm,
    pub debug: String,
}

impl From<RequestError> for AppErr {
    fn from(value: RequestError) -> Self {
        let debug = format!("[{value}]: {value:#?}");
        let worm = match value {
            RequestError::Api(ApiError::BotBlocked) => Worm::Blocked,
            RequestError::Api(ApiError::MessageToDeleteNotFound) => {
                Worm::MessageToDeleteNotFound
            }
            _ => Worm::TxRq(value),
        };
        Self { debug, worm }
    }
}

impl From<reqwest::Error> for AppErr {
    fn from(value: reqwest::Error) -> Self {
        Self {
            debug: format!("reqwest error: {value:?}"),
            worm: Worm::Rqw(value),
        }
    }
}

impl From<DownloadError> for AppErr {
    fn from(value: DownloadError) -> Self {
        Self {
            debug: format!("[{}]: {value:#?}", value),
            worm: Worm::Down(value),
        }
    }
}

impl From<sqlx::Error> for AppErr {
    fn from(value: sqlx::Error) -> Self {
        let debug = format!("[{value}]: {value:#?}");
        match value {
            sqlx::Error::RowNotFound => Self { worm: Worm::NotFound, debug },
            _ => Self { worm: Worm::Sqlx(value), debug },
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppErr {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self { worm: Worm::Unknown, debug: format!("[{value}]: {value:#?}") }
    }
}

macro_rules! err {
    ($worm: ident) => {
        Err(AppErr { worm: crate::error::Worm::$worm, debug: String::new() })
    };
    ($worm: expr) => {
        Err(AppErr { worm: $worm, debug: String::new() })
    };
    ($worm: ident, $debug: literal) => {
        Err(AppErr { worm: crate::error::Worm::$worm, debug: $debug })
    };
    (r, $worm: ident) => {
        AppErr { worm: crate::error::Worm::$worm, debug: String::new() }
    };
    (r, $worm: ident, $debug: literal) => {
        AppErr { worm: crate::error::Worm::$worm, debug: $debug }
    };
}
pub(crate) use err;
