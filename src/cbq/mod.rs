use crate::{
    Ctx, HR, TB,
    book::Book,
    config::Config,
    db::{Flyer, Karbar, KarbarStats, Proxy, Settings, V2ray},
    error::AppErr,
    session::Session,
    state::{AdminGlobal as Ag, KeyData, State, Store, kd},
};
use teloxide::{
    payloads::{SendInvoiceSetters, SendMessageSetters},
    prelude::Requester,
    types::{
        CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup,
        LabeledPrice, MessageId, ParseMode,
    },
};

mod admin;
mod flyer;
mod proxy;
mod v2ray;

pub struct Cbq {
    key: KeyData,
    s: Session,
    // mid: MessageId,
}

impl Cbq {
    pub async fn del_msg(&self) -> HR {
        // self.s.bot.delete_message(self.s.cid, self.mid).await?;
        Ok(())
    }

    pub async fn handle_global(&mut self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::Menu => self.s.send_menu().await?,
            KeyData::Donate => self.s.donate().await?,
            KeyData::GetProxy => self.s.get_proxy().await?,
            KeyData::GetVip => self.s.get_vip().await?,
            KeyData::GetV2ray => self.s.get_v2ray().await?,
            KeyData::MyInviteLinks => self.s.get_invite().await?,
            KeyData::GetFreePoints => self.s.get_free_point().await?,
            KeyData::BuyStarPoints(star) => {
                let points = star * self.s.settings.star_point_price as u32;
                self.s
                    .bot
                    .send_invoice(
                        self.s.cid,
                        format!("{points} امتیاز 🍅"),
                        format!(
                            "خرید {points} امتیاز 🍅 با {star} استار ⭐ تلگرام "
                        ),
                        points.to_string(),
                        "XTR",
                        [LabeledPrice::new("hi", star)],
                    )
                    .start_parameter("x")
                    .await?;
            }
            KeyData::StarPrices => self.s.buy_star_point().await?,
            KeyData::GetRealFreePoints => self.s.get_real_free_point().await?,
            KeyData::ProxyVote(id, vote) => {
                self.del_msg().await?;
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
            KeyData::V2rayVote(id, vote) => {
                self.del_msg().await?;
                let kid = self.s.karbar.tid;
                let vr = V2ray::vote_add(&self.s.ctx, kid, id, vote).await;
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

    pub async fn handle(
        bot: TB, store: Store, ctx: Ctx, q: CallbackQuery,
    ) -> HR {
        let Some(data) = &q.data else { return Ok(()) };
        let Some(msg) = q.regular_message() else { return Ok(()) };

        let settings = Settings::get(&ctx.db).await;
        let conf = Config::get();

        let user = &q.from;
        let karbar = Karbar::init(&ctx, user, "").await?;
        let now = crate::utils::now();
        let cid = msg.chat.id;

        let s = Session { bot, settings, cid, karbar, ctx, conf, now, store };

        if let Err(e) = s.bot.answer_callback_query(q.id.clone()).await {
            match e {
                teloxide::RequestError::Api(_) => {
                    s.send_welcome().await?;
                    s.send_menu().await?;
                    return Ok(());
                }
                _ => Err(e)?,
            }
        };

        let key = KeyData::from(data);
        let state = s.store.get_or_default().await?;
        let is_admin = s.karbar.is_admin();

        let mut cbq = Self {
            // mid: msg.id,
            key,
            s,
        };

        cbq.s.ch_send().await?;
        crate::db::v2ray_auto_update(&mut cbq.s).await;

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
                State::AdminV2rayList => {
                    if cbq.handle_admin_v2ray().await? {
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
