use std::str::FromStr;

use crate::{
    Ctx, HR, TB,
    book::Book,
    config::Config,
    db::{Karbar, Proxy, Settings},
    error::AppErr,
    state::{AdminGlobal as Ag, KeyData, State, Store, kd},
    utils::send_menu,
};
use teloxide::{
    payloads::{CopyMessageSetters, SendMessageSetters},
    prelude::Requester,
    types::{
        CallbackQuery, ChatId, InlineKeyboardButton, InlineKeyboardMarkup,
        Message, MessageId, ParseMode, User,
    },
};

pub struct Cbq {
    bot: TB,
    store: Store,
    ctx: Ctx,
    user: User,
    msg: Message,
    settings: Settings,
    karbar: Karbar,
    is_admin: bool,
    conf: &'static Config,
    key: KeyData,
    state: State,
    now: i64,
    cid: ChatId,
}

impl Cbq {
    pub async fn del_msg(&self) -> HR {
        // self.bot.delete_message(self.cid, self.msg.id).await?;
        Ok(())
    }

    pub async fn handle_global(&mut self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::Menu => {
                send_menu(&self.bot, &self.store, &self.karbar).await?;
            }
            KeyData::GetVip => {
                let Some(msg) = self.settings.vip_msg else {
                    self.bot
                        .send_message(self.cid, "کانفیگ VIP پیدا نشد 😥")
                        .reply_markup(KeyData::main_menu())
                        .await?;
                    return Ok(true);
                };
                let mid = MessageId(msg as i32);
                let kyb = [[
                    InlineKeyboardButton::url(
                        "پروکسی رایگان",
                        self.conf.start_url.clone(),
                    ),
                    InlineKeyboardButton::url(
                        "v2ray رایگان",
                        self.conf.start_url.clone(),
                    ),
                    InlineKeyboardButton::url(
                        "🍌",
                        self.conf.start_url.clone(),
                    ),
                ]];
                self.bot
                    .copy_message(self.cid, self.conf.dev, mid)
                    .reply_markup(InlineKeyboardMarkup::new(kyb))
                    .await?;
            }
            KeyData::GetProxy => {
                let Some(px) = Proxy::get_good(&self.ctx).await else {
                    self.bot
                        .send_message(self.cid, "هیچ پروکسیی یافت نشد 😥")
                        .reply_markup(KeyData::main_menu())
                        .await?;
                    return Ok(true);
                };

                let kid = self.karbar.tid;
                let vote = Proxy::vote_get(&self.ctx, kid, px.id).await;
                let (upp, dnp) = px.up_dn_pct();
                let msg = format!("here is your proxy\n{}", px.url());
                let mut kyb = Vec::new();
                if vote.is_none() {
                    kyb.push(vec![
                        InlineKeyboardButton::callback(
                            format!("{upp}% ({}) 👍", px.up_votes),
                            KeyData::ProxyVote(px.id, 1),
                        ),
                        InlineKeyboardButton::callback(
                            format!("{dnp}% ({}) 👎", px.dn_votes),
                            KeyData::ProxyVote(px.id, -1),
                        ),
                    ]);
                }

                kyb.push(vec![KeyData::main_menu_btn()]);

                self.bot
                    .send_message(self.cid, msg)
                    .reply_markup(InlineKeyboardMarkup::new(kyb))
                    .await?;
            }
            KeyData::GetV2ray => {
                self.bot
                    .send_message(self.cid, "send a v2ray")
                    .reply_markup(KeyData::main_menu())
                    .await?;
            }

            KeyData::MyInviteLinks => {
                let url = format!(
                    "https://t.me/{}?start=inv-{}",
                    self.conf.bot_username, self.karbar.invite_code
                );
                let rurl = reqwest::Url::from_str(&url)
                    .unwrap_or(self.conf.start_url.clone());
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
            }
            KeyData::GetDailyPoints => {
                let last = self.karbar.last_daily_point_at;
                let rem = self.now - last;
                if rem < Config::DAILY_POINTS_DELAY {
                    let wait = Config::DAILY_POINTS_DELAY - rem;
                    let msg = format!("you must wait a {wait} seconds",);
                    self.bot
                        .send_message(self.cid, msg)
                        .reply_markup(KeyData::main_menu())
                        .await?;

                    return Ok(true);
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
            }
            KeyData::ProxyVote(id, vote) => {
                let kid = self.karbar.tid;
                let vr = Proxy::vote_add(&self.ctx, kid, id, vote).await;
                let msg = if vr.is_ok() {
                    "رای شما ثبت شد 🍌"
                } else {
                    "شما قبلا رای داده بودید 🍏"
                };

                self.bot
                    .send_message(self.cid, msg)
                    .reply_markup(KeyData::main_menu())
                    .await?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    async fn admin_proxy_list(&self, page: u32) -> HR {
        let proxies = Proxy::list(&self.ctx, page).await?;
        let count = Proxy::count(&self.ctx).await?;
        let bk = Book::new(proxies, page, count / 32);
        let msg = format!(
            "Proxy List Page\npage: {page} | total: {count}\n\n{}",
            &bk.message()
        );

        self.bot
            .send_message(self.cid, msg)
            .parse_mode(ParseMode::Html)
            .reply_markup(bk.keyboard())
            .await?;
        self.store.update(State::AdminProxyList).await?;

        Ok(())
    }

    async fn set_settings(&self, msg: String, state: State) -> HR {
        self.bot
            .send_message(self.cid, msg)
            .reply_markup(KeyData::main_menu())
            .await?;
        self.store.update(state).await?;
        Ok(())
    }

    pub async fn handle_admin(&self, ag: Ag) -> Result<bool, AppErr> {
        match ag {
            Ag::ForceJoinList => {
                self.bot
                    .send_message(self.cid, "admin force join list")
                    .await?;
            }
            Ag::SendAll => {
                self.bot
                    .send_message(self.cid, "admin send all message")
                    .await?;
            }
            Ag::ProxyList => {
                self.admin_proxy_list(0).await?;
            }
            Ag::V2rayList => {
                self.bot.send_message(self.cid, "admin v2ray list").await?;
            }
            Ag::Settings => {
                let s = &self.settings;

                macro_rules! sbtn {
                    ($ag:ident, $txt:literal) => {
                        InlineKeyboardButton::callback($txt, kd!(ag, Ag::$ag))
                    };
                    ($ag:ident, $txt:literal, $val:ident) => {
                        InlineKeyboardButton::callback(
                            format!($txt, s.$val),
                            kd!(ag, Ag::$ag),
                        )
                    };
                }

                let kyb1 = [
                    sbtn!(SetDailyPt, "پاداش روزانه: {}", daily_points),
                    sbtn!(SetInvitPt, "پاداش دعوت: {}", invite_points),
                    sbtn!(SetVipMsg, "پیام VIP"),
                ];
                let kyb2 = [
                    sbtn!(SetProxyCost, "هزینه پروکسی: {}", proxy_cost),
                    sbtn!(SetV2rayCost, "هزینه v2ray: {}", v2ray_cost),
                    sbtn!(SetVipCost, "هزینه VIP: {}", vip_cost),
                ];

                self.bot
                    .send_message(self.cid, "what do you want to change?")
                    .reply_markup(InlineKeyboardMarkup::new([kyb1, kyb2]))
                    .await?;
            }
            Ag::ProxyDel(page, id) => {
                Proxy::del(&self.ctx, id).await?;
                self.admin_proxy_list(page).await?;
            }
            Ag::ProxyDisabledToggle(page, id) => {
                Proxy::disabled_toggle(&self.ctx, id).await?;
                self.admin_proxy_list(page).await?;
            }
            Ag::ProxyVotesReset(page, id) => {
                Proxy::votes_reset(&self.ctx, id).await?;
                self.admin_proxy_list(page).await?;
            }
            Ag::SetVipCost => {
                let msg = indoc::formatdoc!(
                    "هزینه فعلی پیام VIP: {}
                    
                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.settings.vip_cost
                );
                self.set_settings(msg, State::AdminSetVipCost).await?;
            }
            Ag::SetV2rayCost => {
                let msg = indoc::formatdoc!(
                    "هزینه فعلی کانفیگ v2ray: {}
                    
                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.settings.v2ray_cost
                );
                self.set_settings(msg, State::AdminSetV2rayCost).await?;
            }
            Ag::SetProxyCost => {
                let msg = indoc::formatdoc!(
                    "هزینه فعلی پروکسی: {}
                    
                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.settings.proxy_cost
                );
                self.set_settings(msg, State::AdminSetProxyCost).await?;
            }
            Ag::SetDailyPt => {
                let msg = indoc::formatdoc!(
                    "پاداش روزاه فعلی: {}
                    
                    پاداش جدید را به صورت عدد ارسال کنید:",
                    self.settings.daily_points
                );
                self.set_settings(msg, State::AdminSetDailyPt).await?;
            }
            Ag::SetInvitPt => {
                let msg = indoc::formatdoc!(
                    "پاداش دعوت فعلی: {}
                    
                    پاداش جدید را به صورت عدد ارسال کنید:",
                    self.settings.invite_points
                );
                self.set_settings(msg, State::AdminSetInvitPt).await?;
            }
            Ag::SetVipMsg => {
                let ex = "پیام جدید را ارسال کنید و یا به منوی اصلی بروید";
                let Some(mid) = self.settings.vip_msg else {
                    let m = format!("هیچ پیامی برای VIP تنظیم نشده 🍏\n\n{ex}");
                    self.set_settings(m, State::AdminSetVipMsg).await?;
                    return Ok(true);
                };
                let msg = format!("پیام فعلی VIP 🔽⬇️👇🔻\n\n{ex}");
                self.set_settings(msg, State::AdminSetVipMsg).await?;
            }
        }

        Ok(true)
    }

    pub async fn handle_admin_proxy(&self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::BookAdd => {
                self.bot
                    .send_message(
                        self.cid,
                        concat!(
                            "send a proxy links. each link must be on ",
                            "a different line. like:\n\nproxy 1\nproxy 2\n",
                            "proxy 3.\n\n send in a message or a .txt file"
                        ),
                    )
                    .reply_markup(KeyData::main_menu())
                    .await?;
                self.store.update(State::AdminProxyAdd).await?;
            }
            KeyData::BookItem(page, id) => {
                let px = Proxy::get(&self.ctx, id).await?;
                let (upp, dnp) = px.up_dn_pct();
                let msg = indoc::formatdoc!(
                    r#"
                    <b>Proxy</b>:

                    server: {}
                    port: {}
                    secret: <code>{}</code>
                    
                    <a href="{}">link</a>

                    up votes: {upp}% ({}) 👍
                    down votes: {dnp}% ({}) 👎
                    فعال: {}
                "#,
                    px.server,
                    px.port,
                    px.secret,
                    px.url(),
                    px.up_votes,
                    px.dn_votes,
                    if px.disabled { "❌" } else { "✅" },
                );

                self.bot
                    .send_message(self.cid, msg)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(InlineKeyboardMarkup::new([
                        vec![
                            InlineKeyboardButton::callback(
                                if px.disabled {
                                    "فعال کن"
                                } else {
                                    "غیرفعال کن"
                                },
                                kd!(ag, Ag::ProxyDisabledToggle(page, px.id)),
                            ),
                            InlineKeyboardButton::callback(
                                "ریست کردن رای ها ⚠",
                                kd!(ag, Ag::ProxyVotesReset(page, px.id)),
                            ),
                            InlineKeyboardButton::callback(
                                "حذف کن ⭕",
                                kd!(ag, Ag::ProxyDel(page, px.id)),
                            ),
                        ],
                        vec![
                            InlineKeyboardButton::callback(
                                "<- بازگشت",
                                KeyData::BookPagination(page),
                            ),
                            KeyData::main_menu_btn(),
                        ],
                    ]))
                    .await?;
            }
            KeyData::BookPagination(page) => {
                self.admin_proxy_list(page).await?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    pub async fn handle(
        bot: TB, store: Store, ctx: Ctx, q: CallbackQuery,
    ) -> HR {
        bot.answer_callback_query(q.id.clone()).await?;
        let Some(data) = &q.data else { return Ok(()) };
        let Some(msg) = q.regular_message() else { return Ok(()) };
        let user = &q.from;
        let settings = Settings::get(&ctx.db).await;
        // let conf = Config::get();
        let key = KeyData::from(data);
        let karbar = Karbar::init(&ctx, user, "").await?;
        let state = store.get_or_default().await?;
        let is_admin = karbar.is_admin();
        let now = crate::utils::now();

        let mut cbq = Self {
            conf: Config::get(),
            user: user.clone(),
            settings,
            karbar,
            key,
            state,
            is_admin,
            now,
            cid: msg.chat.id,
            msg: msg.clone(),
            ctx,
            store,
            bot,
        };

        if cbq.handle_global().await? {
            cbq.del_msg().await?;
            return Ok(());
        }

        if let KeyData::Ag(ag) = key {
            if is_admin && cbq.handle_admin(ag).await? {
                cbq.del_msg().await?;
                return Ok(());
            }
        }

        if is_admin {
            match state {
                State::AdminProxyList => {
                    if cbq.handle_admin_proxy().await? {
                        cbq.del_msg().await?;
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
