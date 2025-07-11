use crate::{
    Ctx, HR, TB,
    config::Config,
    db::{Flyer, Karbar, Proxy, Settings},
    state::{AdminGlobal as Ag, KeyData, State, Store, kd, keyboard},
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

    pub async fn donate(&self) -> HR {
        let kyb = InlineKeyboardMarkup::new([[KeyData::main_menu_btn()]]);
        let Some(msg) = self.settings.donate_msg else {
            self.bot
                .send_message(self.cid, "پیام حمایت مالی پیدا نشد 😥")
                .reply_markup(kyb.clone())
                .await?;
            return Ok(());
        };

        let (d, c, m) = (self.conf.dev, self.cid, MessageId(msg as i32));
        self.bot.copy_message(c, d, m).reply_markup(kyb.clone()).await?;

        Ok(())
    }

    pub async fn get_vip(&mut self) -> HR {
        if self.karbar.points < self.settings.vip_cost {
            let m = indoc::indoc!(
                "❌ شما امتیاز کافی برای دریافت کانفیگ VIP ندارید.

            🔒 برای دسترسی به کانفیگ‌های ویژه، امتیاز بیشتری کسب کنید!

            📈 با فعالیت روزانه و دعوت از دوستان، امتیاز شما افزایش می‌یابد."
            );
            self.notify(m).await?;
            return Ok(());
        }

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
            KeyData::donate_url(),
        ]];
        self.bot
            .copy_message(self.cid, self.conf.dev, mid)
            .reply_markup(InlineKeyboardMarkup::new(kyb))
            .await?;

        self.karbar.points -= self.settings.vip_cost;
        self.karbar.set(&self.ctx).await?;

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

        let kb = InlineKeyboardMarkup::new([
            vec![InlineKeyboardButton::url("فعال سازی پروکسی 👘", purl)],
            vec![
                InlineKeyboardButton::url(
                    "دریافت پروکسی و v2ray رایگان 🍓",
                    self.conf.start_url.clone(),
                ),
                KeyData::donate_url(),
            ],
        ]);

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

        let vote = Proxy::vote_get(&self.ctx, self.karbar.tid, px.id).await;
        if vote.is_some() {
            return Ok(());
        }

        self.bot
            .send_message(self.cid, "به این پروکسی رای دهید")
            .reply_markup(InlineKeyboardMarkup::new([
                vec![
                    InlineKeyboardButton::callback(
                        "👍",
                        KeyData::ProxyVote(px.id, 1),
                    ),
                    InlineKeyboardButton::callback(
                        "👎",
                        KeyData::ProxyVote(px.id, -1),
                    ),
                ],
                vec![KeyData::main_menu_btn()],
            ]))
            .await?;

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
            KeyData::donate_url(),
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
            let (wm, wt) = if wait > 3600 {
                ("ساعت", wait / 3600)
            } else if wait > 60 {
                ("دقیقه", wait / 60)
            } else {
                ("ثانیه", wait)
            };
            let msg = format!(
                "{wt} {wm} تا دریافت امتیاز روزانه دیگر باقی مانده است ⏳"
            );
            self.bot
                .send_message(self.cid, msg)
                .reply_markup(KeyData::main_menu())
                .await?;

            return Ok(());
        }

        self.karbar.points += self.settings.daily_points;
        self.karbar.last_daily_point_at = self.now;
        self.karbar.set(&self.ctx).await?;

        let msg = indoc::formatdoc!(
            "{} امتیاز به حساب شما اضافه شد! 🎉
            امتیاز فعلی شما: {} 🍅",
            self.settings.daily_points,
            self.karbar.points
        );

        self.bot
            .send_message(self.cid, msg)
            .reply_markup(KeyData::main_menu())
            .await?;

        Ok(())
    }

    pub async fn send_menu(&self) -> HR {
        let menu_text = indoc::formatdoc!(
            r#"«اینترنت آزاد حق همه مردمه»🌍
            🍅 امتیاز شما: {}
            👥 با دعوت از دوستان و دریافت امتیاز روزانه، امتیاز بیشتری دریافت کن!
        "#,
            self.karbar.points,
        );

        let mut ikb = vec![
            vec![
                InlineKeyboardButton::callback(
                    "امتیاز روزانه 🍅",
                    KeyData::GetDailyPoints,
                ),
                InlineKeyboardButton::callback(
                    "کانفیگ VIP 💎",
                    KeyData::GetVip,
                ),
            ],
            vec![
                InlineKeyboardButton::callback("پروکسی 🛡", KeyData::GetProxy),
                InlineKeyboardButton::callback("V2RAY ⚡️", KeyData::GetV2ray),
            ],
            vec![
                InlineKeyboardButton::callback(
                    "دعوت دوستان و امتیاز گیری 🫂",
                    KeyData::MyInviteLinks,
                ),
                KeyData::donate_btn(),
            ],
        ];

        if self.karbar.is_admin() {
            ikb.push(vec![
                InlineKeyboardButton::callback(
                    "👇 منوی ادمین 👇",
                    KeyData::Unknown,
                ),
                InlineKeyboardButton::callback("کاربر 🔍", kd!(gg, KarbarFind)),
            ]);
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
                InlineKeyboardButton::callback(
                    "لیست تبلیغات",
                    kd!(gg, FlyerList),
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
        let msg = "«ما کانفیگ نمیفروشیم، اینترنت آزاد حق همه مردمه» 🍏🍌";

        let kkb = [
            vec![
                KeyboardButton::new(keyboard::GET_PROXY),
                KeyboardButton::new(keyboard::GET_V2RAY),
                KeyboardButton::new(keyboard::GET_VIP),
            ],
            vec![
                KeyboardButton::new(keyboard::DAILY_PONT),
                KeyboardButton::new(keyboard::INVITE),
                KeyboardButton::new(keyboard::MENU),
            ],
            vec![KeyboardButton::new(keyboard::DONATE)],
        ];

        let kyb = KeyboardMarkup::new(kkb).persistent();

        self.bot.send_message(self.cid, msg).reply_markup(kyb).await?;

        Ok(())
    }

    pub async fn send_karbar(&self, karbar: &Karbar) -> HR {
        fn bol(v: bool) -> &'static str {
            if v { "✅" } else { "❌" }
        }

        let kid = karbar.tid;

        let m = indoc::formatdoc!(
            "نام: {}
            امتیاز: {}
            مسدود است: {}
            بلاک کرده: {}
            ساخت حساب: {}
            اخرین فعالیت: {}

            id: {kid}
            username: {}
            invite code: {}",
            karbar.fullname,
            karbar.points,
            bol(karbar.banned),
            bol(karbar.blocked),
            karbar.created_at,
            karbar.updated_at,
            karbar.username.as_ref().map(|v| v.as_str()).unwrap_or("---"),
            karbar.invite_code
        );

        let kyb = InlineKeyboardMarkup::new([
            vec![
                InlineKeyboardButton::callback(
                    format!("مسدود است {}", bol(karbar.banned)),
                    kd!(ag, Ag::KarbarBanToggle(kid)),
                ),
                InlineKeyboardButton::callback(
                    "تنظیم امتیاز",
                    kd!(ag, Ag::KarbarSetPoints(kid)),
                ),
            ],
            vec![KeyData::main_menu_btn()],
        ]);

        self.bot.send_message(self.cid, m).reply_markup(kyb).await?;

        Ok(())
    }
}
