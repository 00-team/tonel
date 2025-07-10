use crate::config::Config;
use crate::state::{KeyData, State, Store, kd};
use crate::{HR, TB, db::Karbar};
use rand::Rng;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::{payloads::SendMessageSetters, prelude::Requester};

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn random_code() -> String {
    let mut rng = rand::rng();
    let len = rng.random_range(7..=17usize);
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = rng.random_range(0..Config::CODE_ABC.len());
        out.push(Config::CODE_ABC[idx] as char);
    }
    out
}

pub async fn send_menu(bot: &TB, store: &Store, karbar: &Karbar) -> HR {
    let menu_text = format!(
        r#"username: {:?}
points: {}
updated_at: {}
name: {}
invite_code: {}

«ما کانفیگ نمیفروشیم، اینترنت آزاد حق همه مردمه»
🍌
    "#,
        karbar.username,
        karbar.points,
        karbar.updated_at,
        karbar.fullname,
        karbar.invite_code
    );

    let mut keyboard = vec![
        vec![
            InlineKeyboardButton::callback(
                "امتیاز روزانه",
                KeyData::GetDailyPoints,
            ),
            InlineKeyboardButton::callback("کانفیگ VIP 🍓", KeyData::GetVip),
        ],
        vec![
            InlineKeyboardButton::callback("پروکسی", KeyData::GetProxy),
            InlineKeyboardButton::callback("V2ray", KeyData::GetV2ray),
        ],
        vec![InlineKeyboardButton::callback(
            "دعوت دوستان و امتیاز گیری",
            KeyData::MyInviteLinks,
        )],
    ];

    if karbar.is_admin() {
        keyboard.push(vec![
            InlineKeyboardButton::callback(
                "جوین اجباری",
                kd!(gg, ForceJoinList),
            ),
            InlineKeyboardButton::callback("ارسال همهگانی", kd!(gg, SendAll)),
            InlineKeyboardButton::callback("تنظیمات", kd!(gg, Settings)),
        ]);
        keyboard.push(vec![
            InlineKeyboardButton::callback("لیست پروکسی", kd!(gg, ProxyList)),
            InlineKeyboardButton::callback("لیست v2ray", kd!(gg, V2rayList)),
        ]);
    }

    bot.send_message(karbar.cid(), menu_text)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    store.update(State::Menu).await?;

    Ok(())
}
