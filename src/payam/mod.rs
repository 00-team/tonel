use crate::{
    Ctx, HR, TB,
    config::Config,
    db::{Flyer, Karbar, Proxy, Settings, V2ray},
    error::AppErr,
    session::Session,
    state::{AdminGlobal as Ag, KeyData, State, Store, kd, keyboard},
};
use std::str::FromStr;
use teloxide::{
    net::Download,
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message},
};

pub struct Payam {
    msg: Message,
    s: Session,
    state: State,
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
        let now = crate::utils::now();

        let mut payam = Self {
            s: Session { bot, settings, cid, store, karbar, ctx, conf, now },
            state,
            msg,
        };

        if let Some(sp) = payam.msg.successful_payment() {
            let sspp = payam.s.settings.star_point_price as u32;
            let added_points = sspp as u32 * sp.total_amount;
            payam.s.karbar.points += added_points as i64;
            payam.s.karbar.set(&payam.s.ctx).await?;

            let msg = indoc::formatdoc!(
                "از خرید شما سپاس گزاریم 🫠
                
                {} امتیاز به حساب شما اضافه شد! 🎉
                امتیاز فعلی شما: {} 🍅",
                added_points,
                payam.s.karbar.points
            );

            payam
                .s
                .bot
                .send_message(payam.s.cid, msg)
                .reply_markup(KeyData::main_menu())
                .await?;

            return Ok(());
        }

        payam.s.ch_send().await?;

        if is_admin && payam.handle_admin().await? {
            return Ok(());
        }

        let Some(txt) = payam.msg.text() else {
            return Ok(());
        };

        match txt {
            keyboard::GET_VIP => payam.s.get_vip().await?,
            keyboard::INVITE => payam.s.get_invite().await?,
            keyboard::FREE_PONT => payam.s.get_free_point().await?,
            keyboard::GET_V2RAY => payam.s.get_v2ray().await?,
            keyboard::GET_PROXY => payam.s.get_proxy().await?,
            keyboard::MENU => payam.s.send_menu().await?,
            keyboard::BUY_STAR_POINT => payam.s.buy_star_point().await?,
            keyboard::DONATE => payam.s.donate().await?,
            _ => {}
        }

        Ok(())
    }

    async fn gn<T: FromStr>(&self) -> Result<Option<T>, AppErr> {
        let Some(txt) = self.msg.text() else {
            self.s.notify("پیام متنی ندارد ❌").await?;
            return Ok(None);
        };

        let Ok(value) = txt.parse::<T>() else {
            self.s.notify("پیام شما عدد نیست ❌").await?;
            return Ok(None);
        };

        Ok(Some(value))
    }

    async fn handle_admin(&mut self) -> Result<bool, AppErr> {
        macro_rules! set_int {
            ($val:ident) => {{
                let Some(value) = self.gn().await? else {
                    return Ok(true);
                };

                self.s.settings.$val = value;
                self.s.settings.set(&self.s.ctx.db).await?;
                self.s.send_menu().await?;
            }};
        }
        match &self.state {
            State::AdminProxyAdd => self.admin_proxy_add().await?,
            State::AdminV2rayAdd => self.admin_v2ray_add().await?,
            State::AdminSetVipMsg => self.admin_set_vip_msg().await?,
            State::AdminSetDonateMsg => self.admin_set_donate_msg().await?,
            State::AdminFindKarbar => self.admin_find_karbar().await?,
            State::AdminSetVipCost => set_int!(vip_cost),
            State::AdminSetStarPricePt => set_int!(star_point_price),
            State::AdminSetProxyCost => set_int!(proxy_cost),
            State::AdminSetV2rayCost => set_int!(v2ray_cost),
            State::AdminSetInvitPt => set_int!(invite_points),
            State::AdminSetFreePt => set_int!(free_points),
            State::AdminSetFreePtDelay => set_int!(free_point_delay),
            State::AdminSetVipMaxViews => set_int!(vip_max_views),
            State::AdminKarbarSetPoints(kid) => {
                let Some(mv) = self.gn::<i64>().await? else {
                    return Ok(true);
                };
                let k = Karbar::find_with_tid(&self.s.ctx, *kid).await;
                let Some(mut karbar) = k else {
                    self.s.notify("کاربری پیدا نشد 🤡").await?;
                    return Ok(true);
                };
                karbar.points = mv;
                karbar.set(&self.s.ctx).await?;
                self.s.send_karbar(&karbar).await?;
            }

            State::AdminFlyerSetMaxView(id) => {
                let Some(mv) = self.gn::<i64>().await? else {
                    return Ok(true);
                };
                let mut flyer = Flyer::get(&self.s.ctx, *id).await?;
                flyer.max_views = mv.max(-1);
                flyer.set(&self.s.ctx).await?;
                self.s.notify("حداکثر بازدید ثبت شد ✅").await?;
                self.s.store.update(State::Menu).await?;
            }
            State::AdminFlyerSetLabel(id) => {
                let Some(txt) = self.msg.text() else {
                    self.s.notify("پیام متنی ندارد ❌").await?;
                    return Ok(true);
                };
                let mut flyer = Flyer::get(&self.s.ctx, *id).await?;
                flyer.label = txt.to_string();
                flyer.set(&self.s.ctx).await?;
                self.s.notify("عنوان ثبت شد ✅").await?;
                self.s.store.update(State::Menu).await?;
            }
            State::AdminFlyerSetLink(id) => {
                let Some(txt) = self.msg.text() else {
                    self.s.notify("پیام متنی ندارد ❌").await?;
                    return Ok(true);
                };
                let Ok(url) = reqwest::Url::from_str(txt) else {
                    self.s.notify("لینک شما اشتباه است ❌").await?;
                    return Ok(true);
                };
                let mut flyer = Flyer::get(&self.s.ctx, *id).await?;
                flyer.link = Some(url.to_string());
                flyer.set(&self.s.ctx).await?;
                self.s.notify("لینک ثبت شد ✅").await?;
                self.s.store.update(State::Menu).await?;
            }
            State::AdminFlyerAdd => {
                let Some(label) = self.msg.text() else {
                    self.s.notify("پیام شما هیچ متنی ندارد 🍌").await?;
                    return Ok(true);
                };
                let m = indoc::formatdoc!(
                    "نام انتخابی شما: {label}
                        
                        پیام تبلیغ را ارسال کنید"
                );
                let sn =
                    State::AdminFlyerSendMessage { label: label.to_string() };
                self.s.store.update(sn).await?;
                self.s.notify(&m).await?;
            }
            State::AdminFlyerSendMessage { label } => {
                let dev = self.s.conf.dev;
                let (cid, mid) = (self.s.cid, self.msg.id);
                let mx = self.s.bot.forward_message(dev, cid, mid).await?;
                let mut flyer = Flyer::new(label.clone(), mx.id.0 as i64);
                flyer.add(&self.s.ctx).await?;
                let m = concat!(
                    "تبلیغ شما ثبت شد ✅\n\nحداکثر تعداد بازدید ",
                    "را ارسال کنید و یا به منوی اصلی بروید"
                );
                let st = State::AdminFlyerSetMaxView(flyer.id);
                self.s.store.update(st).await?;
                self.s.notify(m).await?;
            }
            State::AdminSendAll => {
                let mid = self.msg.id.0;
                let df = self.msg.forward_origin().is_some();
                let m = concat!(
                    "آیا از ارسال این پیام اطمینان کامل دارید؟\n\n",
                    "⚠ در صورت خطا هیچ گونه امکان توقف ارسال نمی باشد ⚠\n\n",
                    "بعد از تایید. ربات ابتدا پیام را برای شما ارسال می کند",
                    "و سپس برای همه کاربران.\n",
                    "بنابراین، این پیام باید برای خودتان دوبار ارسال شود"
                );
                self.s
                    .bot
                    .send_message(self.s.cid, m)
                    .reply_markup(InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback(
                            "تایید و ارسال ✅",
                            kd!(ag, Ag::SendAllConfirm(df, mid)),
                        ),
                        KeyData::main_menu_btn(),
                    ]]))
                    .await?;
            }
            State::Menu
            | State::AdminFlyerList
            | State::AdminProxyList
            | State::AdminV2rayList => {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn admin_set_vip_msg(&mut self) -> HR {
        let (d, cid, mid) = (self.s.conf.dev, self.s.cid, self.msg.id);
        let mx = self.s.bot.forward_message(d, cid, mid).await?;
        // let mx = self.s.bot.copy_message(d, cid, mid).await?;
        self.s.settings.vip_msg = Some(mx.id.0 as i64);
        self.s.settings.vip_views = 0;
        self.s.settings.set(&self.s.ctx.db).await?;
        self.s.send_menu().await?;
        Ok(())
    }

    async fn admin_set_donate_msg(&mut self) -> HR {
        let (d, cid, mid) = (self.s.conf.dev, self.s.cid, self.msg.id);
        let mx = self.s.bot.forward_message(d, cid, mid).await?;
        // let mx = self.s.bot.copy_message(d, cid, mid).await?;
        self.s.settings.donate_msg = Some(mx.id.0 as i64);
        self.s.settings.set(&self.s.ctx.db).await?;
        self.s.send_menu().await?;
        Ok(())
    }

    async fn admin_find_karbar(&mut self) -> HR {
        let (tid, una) = 'a: {
            if let Some(u) = self.msg.forward_from_user() {
                break 'a (Some(u.id.0 as i64), None);
            }

            let Some(txt) = self.msg.text() else {
                break 'a (None, None);
            };
            let txt = txt.trim();
            if let Some(x) = txt.strip_prefix("@") {
                break 'a (None, Some(x));
            } else if let Ok(id) = txt.parse::<u64>() {
                break 'a (Some(id as i64), None);
            }

            (None, None)
        };

        let karbar = {
            if let Some(tid) = tid {
                Karbar::find_with_tid(&self.s.ctx, tid).await
            } else if let Some(username) = una {
                Karbar::find_with_username(&self.s.ctx, username).await
            } else {
                self.s.notify("هیج ایدیی پیدا نشد 🤡").await?;
                return Ok(());
            }
        };

        let Some(karbar) = karbar else {
            let mut m =
                String::from("هیچ کاربری با این اطلاعات پیدا نشد 🤡\n\n");
            if let Some(tid) = tid {
                m += "id: ";
                m += &tid.to_string();
            }
            if let Some(un) = una {
                m += "username: @";
                m += un;
            }
            self.s.notify(&m).await?;
            return Ok(());
        };

        self.s.send_karbar(&karbar).await?;

        Ok(())
    }

    async fn get_full_text(&self) -> Result<String, AppErr> {
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

        Ok(data)
    }

    async fn admin_proxy_add(&self) -> HR {
        let data = self.get_full_text().await?;
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

        let m = indoc::formatdoc!(
            "{added} پروکسی اضافه شد ✅

            کانفیگ های دیگر را ارسال کنید یه به منوی اصلی بروید"
        );
        self.s
            .bot
            .send_message(self.s.cid, m)
            .reply_markup(KeyData::main_menu())
            .await?;

        Ok(())
    }

    async fn admin_v2ray_add(&self) -> HR {
        let data = self.get_full_text().await?;

        let mut added = 0;

        for line in data.split('\n') {
            if line.is_empty() {
                continue;
            }

            let Some(mut v2) = V2ray::from_link(line) else { continue };
            if v2.add(&self.s.ctx).await.is_ok() {
                added += 1;
            }
        }

        let m = indoc::formatdoc!(
            "{added} کانفیگ v2ray اضافه شد ✅

            کانفیگ های دیگر را ارسال کنید یه به منوی اصلی بروید"
        );
        self.s
            .bot
            .send_message(self.s.cid, m)
            .reply_markup(KeyData::main_menu())
            .await?;

        Ok(())
    }
}
