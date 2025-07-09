use teloxide::dispatching::dialogue;
use teloxide::dispatching::dialogue::ErasedStorage;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub type Store = dialogue::Dialogue<State, ErasedStorage<State>>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyData {
    Unknown,
    GetProxy,
    FreeVpn,
    GetV2ray,
    MyInviteLinks,
    GetDailyPoints,
    Menu,
    Nothing,
    AdminForceJoinList,
    AdminSendAll,
    AdminProxyList,
    AdminV2rayList,
    AdminSetFreeVpn,
    /// set page
    BookPagination(u32),
    BookItem(u32, i64),
    BookAdd,
    AdminProxyAdd,
    AdminProxyDel(u32, i64),
    AdminProxyDisabledToggle(u32, i64),
    // DegreeMajorSelect(u64),
}

impl KeyData {
    pub fn main_menu_btn() -> InlineKeyboardButton {
        InlineKeyboardButton::callback("💼 منو", KeyData::Menu)
    }
    pub fn main_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new([[Self::main_menu_btn()]])
    }
    pub fn nothing() -> InlineKeyboardButton {
        InlineKeyboardButton::callback("👋 هیچی", KeyData::Nothing)
    }
}

impl From<KeyData> for String {
    fn from(value: KeyData) -> Self {
        serde_json::to_string(&value).unwrap()
    }
}

impl From<&str> for KeyData {
    fn from(value: &str) -> Self {
        serde_json::from_str(value).unwrap_or(KeyData::Unknown)
    }
}
impl From<&String> for KeyData {
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Menu,
    AdminProxyList,
    AdminProxyAdd,
}

pub trait CutOff {
    fn cut_off(&mut self, len: usize);
}

impl CutOff for String {
    fn cut_off(&mut self, len: usize) {
        let mut idx = len;
        loop {
            if self.is_char_boundary(idx) {
                break;
            }
            idx -= 1;
        }
        self.truncate(idx)
    }
}

impl CutOff for Option<String> {
    fn cut_off(&mut self, len: usize) {
        if let Some(v) = self {
            v.cut_off(len)
        }
    }
}
