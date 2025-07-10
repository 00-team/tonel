use crate::{
    Ctx, HR, TB,
    config::Config,
    db::{Flyer, Karbar, Proxy, Settings},
    error::AppErr,
    session::Session,
    state::{KeyData, State, Store, keyboard},
    utils,
};
use std::str::FromStr;
use teloxide::{
    net::Download, payloads::SendMessageSetters, prelude::Requester,
    types::Message,
};

pub struct Payam {
    msg: Message,
    s: Session,
}

impl Payam {
    pub async fn handle(bot: TB, store: Store, ctx: Ctx, msg: Message) -> HR {
        let Some(user) = &msg.from else { return Ok(()) };
        let karbar = Karbar::init(&ctx, user, "").await?;
        let state = store.get_or_default().await?;
        let is_admin = karbar.is_admin();
        let conf = Config::get();
        let settings = Settings::get(&ctx.db).await;
        let cid = msg.chat.id;

        let mut payam = Self {
            s: Session {
                bot,
                settings,
                cid,
                store,
                karbar,
                ctx,
                conf,
                now: utils::now(),
            },
            msg,
        };

        async fn gn<T: FromStr>(
            s: &Session, msg: &Message,
        ) -> Result<Option<T>, AppErr> {
            let Some(txt) = msg.text() else {
                s.notify("پیام متنی ندارد ❌").await?;
                return Ok(None);
            };

            let Ok(value) = txt.parse::<T>() else {
                s.notify("پیام شما عدد نیست ❌").await?;
                return Ok(None);
            };

            Ok(Some(value))
        }

        macro_rules! set_int {
            ($val:ident) => {{
                let Some(value) = gn(&payam.s, &payam.msg).await? else {
                    return Ok(());
                };

                payam.s.settings.$val = value;
                payam.s.settings.set(&payam.s.ctx.db).await?;
                payam.s.send_menu().await?;
            }};
        }

        if is_admin {
            match state {
                State::AdminProxyAdd => payam.admin_proxy_add().await?,
                State::AdminSetVipMsg => payam.admin_set_vip_msg().await?,
                State::AdminSetVipCost => set_int!(vip_cost),
                State::AdminSetProxyCost => set_int!(proxy_cost),
                State::AdminSetV2rayCost => set_int!(v2ray_cost),
                State::AdminSetInvitPt => set_int!(invite_points),
                State::AdminSetDailyPt => set_int!(daily_points),
                State::AdminFlyerSetMaxView(id) => {
                    let Some(mv) = gn::<i64>(&payam.s, &payam.msg).await?
                    else {
                        return Ok(());
                    };
                    let mut flyer = Flyer::get(&payam.s.ctx, id).await?;
                    flyer.max_views = mv.max(-1);
                    flyer.set(&payam.s.ctx).await?;
                }
                State::AdminFlyerAdd => {
                    let Some(label) = payam.msg.text() else {
                        payam.s.notify("پیام شما هیچ متنی ندارد 🍌").await?;
                        return Ok(());
                    };
                    let m = indoc::formatdoc!(
                        "نام انتخابی شما: {label}
                        
                        پیام تبلیغ را ارسال کنید"
                    );
                    let sn = State::AdminFlyerSendMessage {
                        label: label.to_string(),
                    };
                    payam.s.store.update(sn).await?;
                    payam.s.notify(&m).await?;
                }
                State::AdminFlyerSendMessage { label } => {
                    let dev = payam.s.conf.dev;
                    let (cid, mid) = (payam.s.cid, payam.msg.id);
                    let mid = payam.s.bot.copy_message(dev, cid, mid).await?;
                    let mut flyer = Flyer::new(label, mid.0 as i64);
                    flyer.add(&payam.s.ctx).await?;
                    let m = concat!(
                        "تبلیغ شما ثبت شد ✅\n\nحداکثر تعداد بازدید ",
                        "را ارسال کنید و یا به منوی اصلی بروید"
                    );
                    let st = State::AdminFlyerSetMaxView(flyer.id);
                    payam.s.store.update(st).await?;
                    payam.s.notify(m).await?;
                }
                State::Menu | State::AdminFlyerList | State::AdminProxyList => {
                }
            }
        }

        let Some(txt) = payam.msg.text() else {
            return Ok(());
        };

        match txt {
            keyboard::GET_VIP => payam.s.get_vip().await?,
            keyboard::INVITE => payam.s.get_invite().await?,
            keyboard::DAILY_PONT => payam.s.get_daily_point().await?,
            keyboard::GET_V2RAY => payam.s.get_v2ray().await?,
            keyboard::GET_PROXY => payam.s.get_proxy().await?,
            keyboard::MENU => payam.s.send_menu().await?,
            _ => {}
        }

        Ok(())
    }

    async fn admin_set_vip_msg(&mut self) -> HR {
        let new_msg = self
            .s
            .bot
            .copy_message(self.s.conf.dev, self.s.cid, self.msg.id)
            .await?;
        self.s.settings.vip_msg = Some(new_msg.0 as i64);
        self.s.settings.set(&self.s.ctx.db).await?;
        self.s.send_menu().await?;
        Ok(())
    }

    async fn admin_proxy_add(&self) -> HR {
        let mut data =
            self.msg.text().map(|v| v.to_string()).unwrap_or_default();

        'd: {
            let Some(doc) = self.msg.document() else { break 'd };
            if doc.file.size > 2 * 1024 * 1024 {
                self.s
                    .bot
                    .send_message(self.s.cid, "max file size is 2MB")
                    .await?;
                break 'd;
            }
            let m = doc.mime_type.clone();

            if !m.map(|v| v.type_() == "text").unwrap_or_default() {
                self.s
                    .bot
                    .send_message(self.s.cid, "only text files are allowed")
                    .await?;
                break 'd;
            }

            let f = self.s.bot.get_file(doc.file.id.clone()).await?;
            let mut buf = Vec::with_capacity(f.size as usize);
            self.s.bot.download_file(&f.path, &mut buf).await?;
            match String::from_utf8(buf.clone()) {
                Ok(v) => data += &v,
                Err(e) => {
                    let nb = buf[..e.utf8_error().valid_up_to()].to_vec();
                    if let Ok(d) = String::from_utf8(nb) {
                        data += &d;
                    }
                }
            }
        };

        let mut added = 0;

        for line in data.split('\n') {
            if line.is_empty() {
                continue;
            }

            let Some(mut px) = Proxy::from_link(line) else { continue };
            if px.add(&self.s.ctx).await.is_ok() {
                added += 1;
            }
        }

        self.s.bot.send_message(
            self.s.cid,
            format!(
                "added {added} new proxies\n\nsend other proxies or go to menu"
            ),
        )
        .reply_markup(KeyData::main_menu())
        .await?;

        Ok(())
    }
}
