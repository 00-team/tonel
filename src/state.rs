use std::error::Error;
use teloxide::dispatching::dialogue;
use teloxide::dispatching::dialogue::ErasedStorage;
use teloxide::macros::BotCommands;

pub type Store = dialogue::Dialogue<State, ErasedStorage<State>>;
pub type HR = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyData {
    Unknown,
    Register,
    Name,
    Phone,
    Age,
    Ssn,
    Complete,
    Cancel,
    IdCard,
    MilitaryService,
    CurrentDegree,
    Degree,
    DegreeMajor,
    DegreeMajorSelect(u64),
    DegreeAssociate,
    DegreeBachelorLinked,
    DegreeBachelorUnLinked,
    DegreeMaster,
    ActionAccept(u64),
    ActionReject(u64),
    ActionClear(u64),
}

impl From<KeyData> for String {
    fn from(value: KeyData) -> Self {
        serde_json::to_string(&value).unwrap()
    }
}

impl From<String> for KeyData {
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap_or(KeyData::Unknown)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    #[default]
    Start,
    UserForm,
    Name,
    Age,
    Ssn,
    Phone,
    Degree,
    DegreeMajor,
    IdCard,
    MilitaryService,
    CurrentDegree,
}

#[derive(Debug, BotCommands, Clone, Copy)]
#[command(rename_rule = "snake_case")]
/// Tonel Bot Commands
pub enum TonelCommands {
    Start,
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
