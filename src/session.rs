use crate::{
    Ctx, HR, TB,
    config::Config,
    db::{Flyer, Karbar, Proxy, Settings, V2ray},
    state::{AdminGlobal as Ag, KeyData, State, Store, kd, keyboard},
};
use std::str::FromStr;
use teloxide::{
    payloads::{CopyMessageSetters, SendMessageSetters},
    prelude::Requester,
    sugar::request::RequestLinkPreviewExt,
    types::{
        ChatId, InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton,
        KeyboardMarkup, MessageId, ParseMode,
    },
    utils::html::escape,
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
    pub async fn ch_send(&mut self) -> HR {
        if self.settings.ch_last_sent + 3600 > self.now {
            return Ok(());
        }
        self.settings.ch_last_sent = self.now;
        self.settings.set(&self.ctx.db).await?;
        let su = &self.conf.start_url;

        let pxs = Proxy::ch_list(&self.ctx).await?;
        let mut kyb1 = Vec::with_capacity(4);
        for px in pxs.iter() {
            let Ok(url) = reqwest::Url::from_str(&px.url()) else { continue };
            kyb1.push(InlineKeyboardButton::url("اتصال", url));
        }

        let kyb2 = vec![
            InlineKeyboardButton::url("پروکسی رایگان", su.clone()),
            InlineKeyboardButton::url("v2ray رایگان", su.clone()),
            KeyData::donate_url(),
        ];

        let kb = InlineKeyboardMarkup::new([kyb1, kyb2]);
        self.bot
            .send_message(
                self.conf.channel,
                "برای پروکسی ها بیشتر به بات مراجعه کنید",
            )
            .reply_markup(kb)
            .await?;

        Ok(())
    }

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

    pub async fn flyer_btn(&mut self) -> Option<InlineKeyboardButton> {
        let mut flyer = Flyer::get_good_link(&self.ctx).await?;

        let u = flyer.link.and_then(|v| reqwest::Url::from_str(&v).ok());
        let Some(url) = u else {
            flyer.link = None;
            let _ = flyer.set(&self.ctx).await;
            return None;
        };

        Some(InlineKeyboardButton::url(flyer.label, url))
    }

    pub async fn get_vip(&mut self) -> HR {
        let cost = self.karbar.calc_cost(self.settings.vip_cost);
        if self.karbar.points < cost {
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
        let mut kyb = vec![vec![
            InlineKeyboardButton::url("پروکسی رایگان", su.clone()),
            InlineKeyboardButton::url("v2ray رایگان", su.clone()),
            KeyData::donate_url(),
        ]];
        if let Some(btn) = self.flyer_btn().await {
            kyb.push(vec![btn]);
        }
        self.bot
            .copy_message(self.cid, self.conf.dev, mid)
            .reply_markup(InlineKeyboardMarkup::new(kyb))
            .await?;

        self.karbar.points -= cost;
        self.karbar.set(&self.ctx).await?;

        self.settings.vip_views += 1;
        self.settings.set(&self.ctx.db).await?;

        Ok(())
    }

    pub async fn get_proxy(&mut self) -> HR {
        let cost = self.karbar.calc_cost(self.settings.proxy_cost);
        if self.karbar.points < cost {
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

        let mut kyb = vec![
            vec![InlineKeyboardButton::url("فعال سازی پروکسی 👘", purl)],
            vec![
                InlineKeyboardButton::url(
                    "دریافت پروکسی و v2ray رایگان 🍓",
                    self.conf.start_url.clone(),
                ),
                KeyData::donate_url(),
            ],
        ];
        if let Some(btn) = self.flyer_btn().await {
            kyb.push(vec![btn]);
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
            let m = "روی دکمه «فعال سازی پروکسی» کلیک کنید.👇";
            self.bot.send_message(self.cid, m).reply_markup(kb).await?;
        }

        self.karbar.points -= cost;
        self.karbar.set(&self.ctx).await?;

        let vote = Proxy::vote_get(&self.ctx, self.karbar.tid, px.id).await;
        if vote.is_some() {
            return Ok(());
        }

        self.bot
            .send_message(self.cid, "به این پروکسی رای دهید")
            .reply_markup(InlineKeyboardMarkup::new([
                [
                    InlineKeyboardButton::callback(
                        "👍",
                        KeyData::ProxyVote(px.id, 1),
                    ),
                    InlineKeyboardButton::callback(
                        "👎",
                        KeyData::ProxyVote(px.id, -1),
                    ),
                ],
                [KeyData::main_menu_btn(), KeyData::donate_btn()],
            ]))
            .await?;

        Ok(())
    }

    pub async fn get_v2ray(&mut self) -> HR {
        let cost = self.karbar.calc_cost(self.settings.v2ray_cost);
        if self.karbar.points < cost {
            self.notify("شما امتیاز کافی برای دریافت v2ray ندارید 🐧").await?;
            return Ok(());
        }

        let mut tries = 0u8;
        let v2 = loop {
            tries += 1;
            if tries > 6 {
                self.notify("هیچ کانفیگ v2ray یافت نشد 😥").await?;
                return Ok(());
            }
            let Some(v2) = V2ray::get_good(&self.ctx).await else { continue };
            break v2;
        };

        let mut kyb = vec![vec![
            InlineKeyboardButton::url(
                "دریافت پروکسی و v2ray رایگان 🍓",
                self.conf.start_url.clone(),
            ),
            KeyData::donate_url(),
        ]];
        if let Some(btn) = self.flyer_btn().await {
            kyb.push(vec![btn]);
        }
        let kb = InlineKeyboardMarkup::new(kyb);

        let m = indoc::formatdoc!(
            r#"<b>کانفیگ v2ray</b>

            <code>{}</code>
            
            همه نت ها 
            حجم 600 گیگ
            
            <a href="https://t.me/xixv2ray/40">آموزش وصل شدن</a>
            
            <a href="https://t.me/xixv2ray/44">برنامه برای اندروید</a>
            
            <a href="https://t.me/xixv2ray/43">برنامه برای آیفون</a>
            
            <a href="https://t.me/proxyxix">گروه پروکسی</a>
            
            «برای پایداری سرور ها به حمایت مالی شما نیاز داریم❤️»"#,
            escape(&v2.link)
        );
        self.bot
            .send_message(self.cid, m)
            .parse_mode(ParseMode::Html)
            .disable_link_preview(true)
            .reply_markup(kb)
            .await?;

        self.karbar.points -= cost;
        self.karbar.set(&self.ctx).await?;

        let vote = V2ray::vote_get(&self.ctx, self.karbar.tid, v2.id).await;
        if vote.is_some() {
            return Ok(());
        }

        let kb = [
            [
                InlineKeyboardButton::callback(
                    "👍",
                    KeyData::V2rayVote(v2.id, 1),
                ),
                InlineKeyboardButton::callback(
                    "👎",
                    KeyData::V2rayVote(v2.id, -1),
                ),
            ],
            [KeyData::main_menu_btn(), KeyData::donate_btn()],
        ];

        self.bot
            .send_message(self.cid, "به این کانفیگ v2ray رای دهید")
            .reply_markup(InlineKeyboardMarkup::new(kb))
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
        let msg = indoc::formatdoc!(
            "🤖 ربات دریافت رایگان کانفیگ V2RAY و پروکسی
            
            🔹 کانفیگ‌های اختصاصی با پینگ تست‌شده ✅
            🔹 پروکسی تلگرام پرسرعت 🟢
            🔹 دسترسی به کانفیگ‌های VIP 👑
            
            
            📥 دریافت از ربات:
            🔗 {url}"
        );

        let kyb = [[
            InlineKeyboardButton::url("پروکسی رایگان", rurl.clone()),
            InlineKeyboardButton::url("v2ray رایگان", rurl.clone()),
            KeyData::donate_url(),
        ]];
        self.bot
            .send_message(self.cid, msg)
            .disable_link_preview(true)
            .reply_markup(InlineKeyboardMarkup::new(kyb))
            .await?;

        Ok(())
    }

    pub async fn get_daily_point(&mut self) -> HR {
        let kb = InlineKeyboardMarkup::new([[InlineKeyboardButton::callback(
            "دریافت امتیاز 🍅",
            KeyData::GetRealDailyPoints,
        )]]);
        let sent = 'a: {
            let Some(mut flyer) = Flyer::get_good(&self.ctx).await else {
                break 'a false;
            };
            let m = MessageId(flyer.mid as i32);
            let (d, c) = (self.conf.dev, self.cid);

            let r = self.bot.copy_message(c, d, m).reply_markup(kb);

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
            self.get_real_daily_point().await?;
        }

        Ok(())
    }

    pub async fn get_real_daily_point(&mut self) -> HR {
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
            r#"🌍 «اینترنت آزاد حق همه مردمه» 

            🍅 امتیاز شما: {}

            👥 با دعوت از دوستان و دریافت امتیاز روزانه، امتیاز بیشتری دریافت کن!"#,
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
        let msg = "آماده‌ی خدمات‌رسانی ۲۴ ساعته به شما هستیم! 🕒✨";

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

        let kyb = KeyboardMarkup::new(kkb).resize_keyboard();

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
            karbar.username.as_deref().unwrap_or("---"),
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
