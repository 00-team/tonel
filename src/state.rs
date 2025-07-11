use crate::config::Config;
use teloxide::dispatching::dialogue;
use teloxide::dispatching::dialogue::ErasedStorage;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub type Store = dialogue::Dialogue<State, ErasedStorage<State>>;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum KeyData {
    Unknown,
    // global
    Menu,
    Donate,
    GetProxy,
    GetVip,
    GetV2ray,
    MyInviteLinks,
    GetDailyPoints,
    ProxyVote(i64, i8),
    // admin global
    Ag(AdminGlobal),

    /// set page
    BookPagination(u32),
    BookItem(u32, i64),
    BookAdd,
}

pub mod keyboard {
    pub const GET_PROXY: &str = "پروکسی";
    pub const GET_VIP: &str = "کانفیگ VIP 🍓";
    pub const GET_V2RAY: &str = "V2ray";
    pub const DAILY_PONT: &str = "امتیاز روزانه 🍅";
    pub const INVITE: &str = "دعوت دوستان";
    pub const MENU: &str = "منو";
    pub const DONATE: &str = "حمایت مالی 💰";
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AdminGlobal {
    ForceJoinList,
    KarbarFind,
    KarbarSetPoints(i64),
    KarbarBanToggle(i64),
    SendAll,
    SendAllConfirm(bool, i32),
    ProxyList,
    V2rayList,
    Settings,
    ProxyDel(u32, i64),
    ProxyVotesReset(u32, i64),
    ProxyDisabledToggle(u32, i64),
    SetDailyPt,
    SetInvitPt,
    SetProxyCost,
    SetV2rayCost,
    SetVipCost,
    SetVipMsg,
    SetDonateMsg,
    FlyerList,
    FlyerDel(u32, i64),
    FlyerViewsReset(u32, i64),
    FlyerDisabledToggle(u32, i64),
    FlyerSetMaxViews(u32, i64),
}

macro_rules! kd {
    (gg, $ident:ident) => {
        crate::state::KeyData::Ag(crate::state::AdminGlobal::$ident)
    };
    (ag, $ex:expr) => {
        crate::state::KeyData::Ag($ex)
    };
    ($ident:ident) => {
        crate::state::KeyData::$ident
    };
}
pub(crate) use kd;

impl KeyData {
    pub fn main_menu_btn() -> InlineKeyboardButton {
        InlineKeyboardButton::callback("💼 منو", KeyData::Menu)
    }
    pub fn donate_url() -> InlineKeyboardButton {
        InlineKeyboardButton::url(
            "حمایت مالی 💰",
            Config::get().donate_url.clone(),
        )
    }
    pub fn donate_btn() -> InlineKeyboardButton {
        InlineKeyboardButton::callback("حمایت مالی 💰", KeyData::Donate)
    }

    pub fn main_menu() -> InlineKeyboardMarkup {
        InlineKeyboardMarkup::new([[Self::main_menu_btn(), Self::donate_btn()]])
    }

    // pub fn nothing() -> InlineKeyboardButton {
    //     InlineKeyboardButton::callback("👋 هیچی", KeyData::Nothing)
    // }
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

#[derive(Debug, Default, serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Menu,
    AdminSendAll,
    AdminFindKarbar,
    AdminKarbarSetPoints(i64),

    AdminProxyList,
    AdminProxyAdd,

    AdminFlyerList,
    AdminFlyerAdd,
    AdminFlyerSendMessage {
        label: String,
    },
    AdminFlyerSetMaxView(i64),

    AdminSetDailyPt,
    AdminSetInvitPt,
    AdminSetProxyCost,
    AdminSetV2rayCost,
    AdminSetVipCost,
    AdminSetVipMsg,
    AdminSetDonateMsg,
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
