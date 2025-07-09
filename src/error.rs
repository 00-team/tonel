use std::fmt::Debug;
use teloxide::{DownloadError, RequestError};

#[derive(Debug)]
pub enum Worm {
    Unknown,
    NotFound,
    AlreadyExists,
    Banned,
    TxRq(RequestError),
    Sqlx(sqlx::Error),
    Down(DownloadError),
}

#[derive(Debug)]
pub struct AppErr {
    pub worm: Worm,
    pub debug: String,
}

impl From<RequestError> for AppErr {
    fn from(value: RequestError) -> Self {
        Self {
            debug: format!("[{}]: {value:#?}", value.to_string()),
            worm: Worm::TxRq(value),
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
        let debug = format!("[{}]: {value:#?}", value.to_string());
        match value {
            sqlx::Error::RowNotFound => Self { worm: Worm::NotFound, debug },
            _ => Self { worm: Worm::Sqlx(value), debug },
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppErr {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self {
            worm: Worm::Unknown,
            debug: format!("[{}]: {value:#?}", value.to_string()),
        }
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
