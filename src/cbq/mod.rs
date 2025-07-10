use crate::{
    Ctx, HR, TB,
    book::Book,
    config::Config,
    db::{Flyer, Karbar, Proxy, Settings},
    error::AppErr,
    session::Session,
    state::{AdminGlobal as Ag, KeyData, State, Store, kd},
};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{
        CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, MessageId,
        ParseMode,
    },
};

mod flyer;
mod proxy;

pub struct Cbq {
    key: KeyData,
    s: Session,
    mid: MessageId,
}

impl Cbq {
    pub async fn del_msg(&self) -> HR {
        self.s.bot.delete_message(self.s.cid, self.mid).await?;
        Ok(())
    }

    pub async fn handle_global(&mut self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::Menu => self.s.send_menu().await?,
            KeyData::GetProxy => self.s.get_proxy().await?,
            KeyData::GetVip => self.s.get_vip().await?,
            KeyData::GetV2ray => self.s.get_v2ray().await?,
            KeyData::MyInviteLinks => self.s.get_invite().await?,
            KeyData::GetDailyPoints => self.s.get_daily_point().await?,
            KeyData::ProxyVote(id, vote) => {
                let kid = self.s.karbar.tid;
                let vr = Proxy::vote_add(&self.s.ctx, kid, id, vote).await;
                let msg = if vr.is_ok() {
                    "رای شما ثبت شد 🍌"
                } else {
                    "شما قبلا رای داده بودید 🍏"
                };

                self.s
                    .bot
                    .send_message(self.s.cid, msg)
                    .reply_markup(KeyData::main_menu())
                    .await?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    async fn set_settings(&self, msg: String, state: State) -> HR {
        self.s
            .bot
            .send_message(self.s.cid, msg)
            .reply_markup(KeyData::main_menu())
            .await?;
        self.s.store.update(state).await?;
        Ok(())
    }

    pub async fn handle_admin(&self, ag: Ag) -> Result<bool, AppErr> {
        match ag {
            Ag::ForceJoinList => {
                self.s
                    .bot
                    .send_message(self.s.cid, "admin force join list")
                    .await?;
            }
            Ag::SendAll => {
                self.s
                    .bot
                    .send_message(self.s.cid, "admin send all message")
                    .await?;
            }
            Ag::ProxyList => self.admin_proxy_list(0).await?,
            Ag::FlyerList => self.admin_flyer_list(0).await?,
            Ag::V2rayList => {
                self.s.bot.send_message(self.s.cid, "admin v2ray list").await?;
            }
            Ag::Settings => {
                let s = &self.s.settings;

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

                self.s
                    .bot
                    .send_message(self.s.cid, "what do you want to change?")
                    .reply_markup(InlineKeyboardMarkup::new([kyb1, kyb2]))
                    .await?;
            }
            Ag::ProxyDel(page, id) => {
                Proxy::del(&self.s.ctx, id).await?;
                self.admin_proxy_list(page).await?;
            }
            Ag::ProxyDisabledToggle(page, id) => {
                Proxy::disabled_toggle(&self.s.ctx, id).await?;
                self.admin_proxy_list(page).await?;
            }
            Ag::ProxyVotesReset(page, id) => {
                Proxy::votes_reset(&self.s.ctx, id).await?;
                self.admin_proxy_list(page).await?;
            }
            Ag::FlyerSetMaxViews(_page, id) => {
                let msg = concat!(
                    "حداکثر تعداد بازدید را به صورت عدد ارسال کنید\n",
                    "عداد زیر -1 به معنی بی نهایت تفسیر می شوند"
                );
                self.s.notify(msg).await?;
                self.s.store.update(State::AdminFlyerSetMaxView(id)).await?;
            }
            Ag::FlyerDel(page, id) => {
                Flyer::del(&self.s.ctx, id).await?;
                self.admin_flyer_list(page).await?;
            }
            Ag::FlyerDisabledToggle(page, id) => {
                let mut flyer = Flyer::get(&self.s.ctx, id).await?;
                flyer.disabled = !flyer.disabled;
                flyer.set(&self.s.ctx).await?;
                self.admin_flyer_list(page).await?;
            }
            Ag::FlyerViewsReset(page, id) => {
                let mut flyer = Flyer::get(&self.s.ctx, id).await?;
                flyer.views = 0;
                flyer.set(&self.s.ctx).await?;
                self.admin_flyer_list(page).await?;
            }
            Ag::SetVipCost => {
                let msg = indoc::formatdoc!(
                    "هزینه فعلی پیام VIP: {}
                    
                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.vip_cost
                );
                self.set_settings(msg, State::AdminSetVipCost).await?;
            }
            Ag::SetV2rayCost => {
                let msg = indoc::formatdoc!(
                    "هزینه فعلی کانفیگ v2ray: {}
                    
                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.v2ray_cost
                );
                self.set_settings(msg, State::AdminSetV2rayCost).await?;
            }
            Ag::SetProxyCost => {
                let msg = indoc::formatdoc!(
                    "هزینه فعلی پروکسی: {}
                    
                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.proxy_cost
                );
                self.set_settings(msg, State::AdminSetProxyCost).await?;
            }
            Ag::SetDailyPt => {
                let msg = indoc::formatdoc!(
                    "پاداش روزاه فعلی: {}
                    
                    پاداش جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.daily_points
                );
                self.set_settings(msg, State::AdminSetDailyPt).await?;
            }
            Ag::SetInvitPt => {
                let msg = indoc::formatdoc!(
                    "پاداش دعوت فعلی: {}
                    
                    پاداش جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.invite_points
                );
                self.set_settings(msg, State::AdminSetInvitPt).await?;
            }
            Ag::SetVipMsg => {
                let ex = "پیام جدید را ارسال کنید و یا به منوی اصلی بروید";
                let Some(mid) = self.s.settings.vip_msg else {
                    let m = format!("هیچ پیامی برای VIP تنظیم نشده 🍏\n\n{ex}");
                    self.set_settings(m, State::AdminSetVipMsg).await?;
                    return Ok(true);
                };
                let msg = format!("پیام فعلی VIP 🔽⬇️👇🔻\n\n{ex}");
                self.set_settings(msg, State::AdminSetVipMsg).await?;
                let mid = MessageId(mid as i32);
                self.s
                    .bot
                    .copy_message(self.s.cid, self.s.conf.dev, mid)
                    .await?;
            }
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
        let conf = Config::get();
        let key = KeyData::from(data);
        let karbar = Karbar::init(&ctx, user, "").await?;
        let state = store.get_or_default().await?;
        let is_admin = karbar.is_admin();
        let now = crate::utils::now();
        let cid = msg.chat.id;

        let mut cbq = Self {
            mid: msg.id,
            key,
            s: Session { bot, settings, cid, karbar, ctx, conf, now, store },
        };

        if cbq.handle_global().await? {
            return Ok(());
        }

        if let KeyData::Ag(ag) = key {
            if is_admin && cbq.handle_admin(ag).await? {
                return Ok(());
            }
        }

        if is_admin {
            match state {
                State::AdminProxyList => {
                    if cbq.handle_admin_proxy().await? {
                        return Ok(());
                    }
                }
                State::AdminFlyerList => {
                    if cbq.handle_admin_flyer().await? {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
