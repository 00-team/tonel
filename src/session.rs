use crate::{
    Ctx, HR, TB,
    config::Config,
    db::{Flyer, Karbar, Proxy, Settings},
    state::{KeyData, State, Store, kd, keyboard},
};
use std::str::FromStr;
use teloxide::{
    payloads::{CopyMessageSetters, SendMessageSetters},
    prelude::Requester,
    types::{
        ChatId, InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton,
        KeyboardMarkup, MessageId,
    },
};

pub struct Session {
    pub bot: TB,
    pub settings: Settings,
    pub cid: ChatId,
    pub karbar: Karbar,
    pub ctx: Ctx,
    pub conf: &'static Config,
    pub now: i64,
    pub store: Store,
}

impl Session {
    pub async fn notify(&self, text: &str) -> HR {
        self.bot
            .send_message(self.cid, text)
            .reply_markup(KeyData::main_menu())
            .await?;
        Ok(())
    }

    pub async fn get_vip(&self) -> HR {
        let Some(msg) = self.settings.vip_msg else {
            self.bot
                .send_message(self.cid, "کانفیگ VIP پیدا نشد 😥")
                .reply_markup(KeyData::main_menu())
                .await?;
            return Ok(());
        };
        let su = &self.conf.start_url;
        let mid = MessageId(msg as i32);
        let kyb = [[
            InlineKeyboardButton::url("پروکسی رایگان", su.clone()),
            InlineKeyboardButton::url("v2ray رایگان", su.clone()),
            InlineKeyboardButton::url("🍌", su.clone()),
        ]];
        self.bot
            .copy_message(self.cid, self.conf.dev, mid)
            .reply_markup(InlineKeyboardMarkup::new(kyb))
            .await?;

        Ok(())
    }

    pub async fn get_proxy(&mut self) -> HR {
        if self.karbar.points < self.settings.proxy_cost {
            self.notify("شما امتیاز کافی برای دریافت پروکسی ندارید 🐧").await?;
            return Ok(());
        }

        let mut tries = 0u8;
        let (px, purl) = loop {
            tries += 1;
            if tries > 6 {
                self.notify("هیچ پروکسیی یافت نشد 😥").await?;
                return Ok(());
            }
            let Some(px) = Proxy::get_good(&self.ctx).await else { continue };
            let Ok(purl) = reqwest::Url::from_str(&px.url()) else {
                Proxy::disabled_toggle(&self.ctx, px.id).await?;
                continue;
            };

            break (px, purl);
        };

        let vote = Proxy::vote_get(&self.ctx, self.karbar.tid, px.id).await;
        let mut kyb = Vec::with_capacity(3);
        kyb.push(vec![InlineKeyboardButton::url("فعال سازی پروکسی 👘", purl)]);
        if vote.is_none() {
            kyb.push(vec![
                InlineKeyboardButton::callback(
                    "👍",
                    KeyData::ProxyVote(px.id, 1),
                ),
                InlineKeyboardButton::callback(
                    "👎",
                    KeyData::ProxyVote(px.id, -1),
                ),
            ]);
        }

        let kb = InlineKeyboardMarkup::new(kyb);

        let sent = 'a: {
            let Some(mut flyer) = Flyer::get_good(&self.ctx).await else {
                break 'a false;
            };
            let m = MessageId(flyer.mid as i32);
            let (d, c) = (self.conf.dev, self.cid);

            let r = self.bot.copy_message(c, d, m).reply_markup(kb.clone());

            if r.await.is_err() {
                flyer.disabled = true;
                let _ = flyer.set(&self.ctx).await;
                break 'a false;
            }

            flyer.views += 1;
            let _ = flyer.set(&self.ctx).await;

            true
        };

        if !sent {
            self.bot
                .send_message(self.cid, "هیچ تبلیغی پیدا نشد. پیام پیشفرض")
                .reply_markup(kb)
                .await?;
        }

        self.karbar.points -= self.settings.proxy_cost;
        self.karbar.set(&self.ctx).await?;

        Ok(())
    }

    pub async fn get_v2ray(&self) -> HR {
        self.bot
            .send_message(self.cid, "send a v2ray")
            .reply_markup(KeyData::main_menu())
            .await?;

        Ok(())
    }

    pub async fn get_invite(&self) -> HR {
        let url = format!(
            "https://t.me/{}?start=inv-{}",
            self.conf.bot_username, self.karbar.invite_code
        );
        let rurl =
            reqwest::Url::from_str(&url).unwrap_or(self.conf.start_url.clone());
        let msg = indoc::formatdoc!("your invite link: {url}",);
        let kyb = [[
            InlineKeyboardButton::url("پروکسی رایگان", rurl.clone()),
            InlineKeyboardButton::url("v2ray رایگان", rurl.clone()),
            InlineKeyboardButton::url("🍌", rurl.clone()),
        ]];
        self.bot
            .send_message(self.cid, msg)
            .reply_markup(InlineKeyboardMarkup::new(kyb))
            .await?;

        Ok(())
    }

    pub async fn get_daily_point(&mut self) -> HR {
        let rem = self.now - self.karbar.last_daily_point_at;
        if rem < Config::DAILY_POINTS_DELAY {
            let wait = Config::DAILY_POINTS_DELAY - rem;
            let msg = format!("you must wait a {wait} seconds",);
            self.bot
                .send_message(self.cid, msg)
                .reply_markup(KeyData::main_menu())
                .await?;

            return Ok(());
        }

        self.karbar.points += self.settings.daily_points;
        self.karbar.last_daily_point_at = self.now;
        self.karbar.set(&self.ctx).await?;

        let msg = format!(
            "{} points added to your wallet\ncurrnet points: {}",
            self.settings.daily_points, self.karbar.points
        );

        self.bot
            .send_message(self.cid, msg)
            .reply_markup(KeyData::main_menu())
            .await?;

        Ok(())
    }

    pub async fn send_menu(&self) -> HR {
        let menu_text = format!(
            r#"username: {:?}
points: {}
updated_at: {}
name: {}
invite_code: {}

«ما کانفیگ نمیفروشیم، اینترنت آزاد حق همه مردمه»
🍌
    "#,
            self.karbar.username,
            self.karbar.points,
            self.karbar.updated_at,
            self.karbar.fullname,
            self.karbar.invite_code
        );

        let mut ikb = vec![
            vec![
                InlineKeyboardButton::callback(
                    "امتیاز روزانه",
                    KeyData::GetDailyPoints,
                ),
                InlineKeyboardButton::callback(
                    "کانفیگ VIP 🍓",
                    KeyData::GetVip,
                ),
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

        if self.karbar.is_admin() {
            ikb.push(vec![
                InlineKeyboardButton::callback(
                    "جوین اجباری",
                    kd!(gg, ForceJoinList),
                ),
                InlineKeyboardButton::callback(
                    "ارسال همهگانی",
                    kd!(gg, SendAll),
                ),
                InlineKeyboardButton::callback("تنظیمات", kd!(gg, Settings)),
            ]);
            ikb.push(vec![
                InlineKeyboardButton::callback(
                    "لیست پروکسی",
                    kd!(gg, ProxyList),
                ),
                InlineKeyboardButton::callback(
                    "لیست v2ray",
                    kd!(gg, V2rayList),
                ),
            ]);
        }

        self.bot
            .send_message(self.karbar.cid(), menu_text)
            .reply_markup(InlineKeyboardMarkup::new(ikb))
            .await?;

        self.store.update(State::Menu).await?;

        Ok(())
    }

    pub async fn send_welcome(&self) -> HR {
        let msg = "«ما کانفیگ نمیفروشیم، اینترنت آزاد حق همه مردمه» 🍌";

        let kkb = [
            [
                KeyboardButton::new(keyboard::GET_PROXY),
                KeyboardButton::new(keyboard::GET_V2RAY),
                KeyboardButton::new(keyboard::GET_VIP),
            ],
            [
                KeyboardButton::new(keyboard::DAILY_PONT),
                KeyboardButton::new(keyboard::INVITE),
                KeyboardButton::new(keyboard::MENU),
            ],
        ];

        let kyb = KeyboardMarkup::new(kkb).persistent();

        self.bot.send_message(self.cid, msg).reply_markup(kyb).await?;

        Ok(())
    }
}
