use super::*;

impl super::Cbq {
    pub async fn handle_admin(&self, ag: Ag) -> Result<bool, AppErr> {
        match ag {
            Ag::ForceJoinList => {
                let mut kyb = Vec::with_capacity(self.s.conf.force_join.len());
                for (_, title, url) in self.s.conf.force_join.iter() {
                    kyb.push([InlineKeyboardButton::url(title, url.clone())]);
                }

                self.s
                    .bot
                    .send_message(self.s.cid, "admin force join list")
                    .reply_markup(InlineKeyboardMarkup::new(kyb))
                    .await?;
            }
            Ag::KarbarFind => {
                let m = concat!(
                    "پیدا کردن کاربر 🔍\n\n",
                    "1. ایدی عددی او را ارسال کنید\n",
                    "2. ایدی اصلی او را با @ ارسال کنید\n",
                    "3. پیامی از او ارسال کنید\n"
                );
                self.s.store.update(State::AdminFindKarbar).await?;
                self.s.notify(m).await?;
            }
            Ag::KarbarBanToggle(kid) => {
                if kid == self.s.karbar.tid {
                    self.s.notify("خود را نمی توان مسدود کرد 🤡").await?;
                    return Ok(true);
                }
                let ctx = &self.s.ctx;
                let Some(mut k) = Karbar::find_with_tid(ctx, kid).await else {
                    self.s.notify("کاربری برای مسدود کردن پیدا نشد 🤡").await?;
                    return Ok(true);
                };

                k.banned = !k.banned;
                k.set(ctx).await?;
                self.s.send_karbar(&k).await?;
            }
            Ag::KarbarSetPoints(kid) => {
                self.s.store.update(State::AdminKarbarSetPoints(kid)).await?;
                self.s.notify("تعداد امتیاز را به صورت عدد ارسال کنید").await?;
            }
            Ag::SendAll => {
                let stats = KarbarStats::get(&self.s.ctx).await;
                let stats = stats.unwrap_or_default();
                let avg_point = if stats.total > 0 && stats.total_points > 0 {
                    stats.total_points as f64 / stats.total as f64
                } else {
                    -1.0
                };
                let msg = indoc::formatdoc! {
                    "ارسال همهگانی 🧆

                    تعداد کاربران: {}
                    تعداد کاربرانی که بات را بلاک کرده اند: {}
                    تعداد کاربران فعال در ۵ ساعت گذشته: {}
                    تعداد کاربران فعال در ۷ روز گذشته: {}
                    جمع امتیاز: {}
                    میانگین امتیاز هر کاربر: {avg_point:.2}

                    پیام خود را ارسال کنید:
                ",
                    stats.total, stats.blocked, stats.active_5h,
                    stats.active_7d, stats.total_points,
                };
                self.s.store.update(State::AdminSendAll).await?;
                self.s.notify(&msg).await?;
            }
            Ag::ProxyList => self.admin_proxy_list(0).await?,
            Ag::FlyerList => self.admin_flyer_list(0).await?,
            Ag::V2rayList => self.admin_v2ray_list(0).await?,
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
                    sbtn!(SetFreePt, "پاداش رایگان: {}", free_points),
                    sbtn!(SetInvitPt, "پاداش دعوت: {}", invite_points),
                    sbtn!(SetVipMsg, "پیام VIP"),
                ];
                let kyb2 = [
                    sbtn!(SetProxyCost, "هزینه پروکسی: {}", proxy_cost),
                    sbtn!(SetV2rayCost, "هزینه v2ray: {}", v2ray_cost),
                    sbtn!(SetVipCost, "هزینه VIP: {}", vip_cost),
                ];
                let kyb3 = [
                    sbtn!(SetDonateMsg, "پیام حمایت مالی"),
                    sbtn!(SetVipMaxViews, "بازدید VIP: {}", vip_max_views),
                    KeyData::main_menu_btn(),
                ];
                let kyb4 = [
                    sbtn!(
                        SetFreePtDelay,
                        "تاخیر پاداش رایگان {}",
                        free_point_delay
                    ),
                    sbtn!(
                        SetStarPricePt,
                        "پاداش برای یک استار {}",
                        star_point_price
                    ),
                    KeyData::main_menu_btn(),
                ];
                let kb = InlineKeyboardMarkup::new([kyb1, kyb2, kyb3, kyb4]);

                let m = indoc::formatdoc!(
                    "تنظیمات ⚙️
                
                    بازدید از VIP: {}",
                    self.s.settings.vip_views
                );
                self.s.bot.send_message(self.s.cid, m).reply_markup(kb).await?;
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
            Ag::ProxyDeleteAllConfirm => {
                Proxy::del_all(&self.s.ctx).await?;
                self.s.send_menu().await?;
            }
            Ag::V2rayDel(page, id) => {
                V2ray::del(&self.s.ctx, id).await?;
                self.admin_v2ray_list(page).await?;
            }
            Ag::V2rayDisabledToggle(page, id) => {
                V2ray::disabled_toggle(&self.s.ctx, id).await?;
                self.admin_v2ray_list(page).await?;
            }
            Ag::V2rayVotesReset(page, id) => {
                V2ray::votes_reset(&self.s.ctx, id).await?;
                self.admin_v2ray_list(page).await?;
            }
            Ag::V2rayDeleteAllConfirm => {
                V2ray::del_all(&self.s.ctx).await?;
                self.s.send_menu().await?;
            }
            Ag::FlyerSetMaxViews(_page, id) => {
                let msg = concat!(
                    "حداکثر تعداد بازدید را به صورت عدد ارسال کنید\n",
                    "عداد زیر -1 به معنی بی نهایت تفسیر می شوند"
                );
                self.s.notify(msg).await?;
                self.s.store.update(State::AdminFlyerSetMaxView(id)).await?;
            }
            Ag::FlyerSetLink(_page, id) => {
                let msg = "لینک تبلیغ را ارسال کنید 🔗";
                self.s.notify(msg).await?;
                self.s.store.update(State::AdminFlyerSetLink(id)).await?;
            }
            Ag::FlyerSetLabel(_page, id) => {
                let msg = "عنوان تبلیغ را ارسال کنید 🏷️";
                self.s.notify(msg).await?;
                self.s.store.update(State::AdminFlyerSetLabel(id)).await?;
            }
            Ag::FlyerDelLink(page, id) => {
                let mut flyer = Flyer::get(&self.s.ctx, id).await?;
                flyer.link = None;
                flyer.set(&self.s.ctx).await?;

                let msg = "لینک تبلیغ حذف شد 🍌";
                self.s.notify(msg).await?;
                self.admin_flyer_list(page).await?;
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
            Ag::FlyerDeleteAllConfirm => {
                Flyer::del_all(&self.s.ctx).await?;
                self.s.send_menu().await?;
            }
            Ag::SetVipMaxViews => {
                let msg = indoc::formatdoc!(
                    "حداکثر بازدید پیام VIP: {}
                    
                    مقدار -1 به معنا بینهایت می باشد ♾️
                    مقدار جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.vip_cost
                );
                self.set_settings(msg, State::AdminSetVipMaxViews).await?;
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
            Ag::SetStarPricePt => {
                let msg = indoc::formatdoc!(
                    "پاداش برای هر یک استار فعلی: {}
                    
                    توجه داشته باشید که این قیمت فقط برای 1 استار می باشد
                    ربات برااساس این قیمت به کاربر امتیاز می دهد
                    به عنوان مثال اگر قیمت هر استار 2 امتیاز باشد
                    خرید 10 استار ⭐ برار 20 امتیاز 🍅 می باشد

                    هزینه جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.star_point_price
                );
                self.set_settings(msg, State::AdminSetStarPricePt).await?;
            }
            Ag::SetFreePtDelay => {
                let msg = indoc::formatdoc!(
                    "تاخیر پاداش رایگان فعلی: {}
                    
                    تاخیر پاداش جدید را به صورت ثانیه ارسال کنید:

                    مثال:
                    
                    1 ساعت = 3600
                    3 ساعت = 10800
                    12 ساعت = 43200
                    24 ساعت = 86400",
                    self.s.settings.free_points
                );
                self.set_settings(msg, State::AdminSetFreePtDelay).await?;
            }
            Ag::SetFreePt => {
                let msg = indoc::formatdoc!(
                    "پاداش رایگان فعلی: {}
                    
                    پاداش جدید را به صورت عدد ارسال کنید:",
                    self.s.settings.free_points
                );
                self.set_settings(msg, State::AdminSetFreePt).await?;
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
                let dev = self.s.conf.dev;
                self.s.bot.forward_message(self.s.cid, dev, mid).await?;
            }
            Ag::SetDonateMsg => {
                let ex = "پیام جدید را ارسال کنید و یا به منوی اصلی بروید";
                let Some(mid) = self.s.settings.vip_msg else {
                    let m = format!(
                        "هیچ پیامی برای حمایت مالی تنظیم نشده 🍏\n\n{ex}"
                    );
                    self.set_settings(m, State::AdminSetDonateMsg).await?;
                    return Ok(true);
                };
                let msg = format!("پیام فعلی حمایت مالی 🔽⬇️👇🔻\n\n{ex}");
                self.set_settings(msg, State::AdminSetDonateMsg).await?;
                let mid = MessageId(mid as i32);
                let dev = self.s.conf.dev;
                self.s.bot.forward_message(self.s.cid, dev, mid).await?;
            }
            Ag::SendAllConfirm(df, mid) => {
                let bcid = self.s.cid;
                let bot = self.s.bot.clone();
                let ctx = self.s.ctx.clone();
                let bmid = MessageId(mid);

                if df {
                    bot.forward_message(self.s.cid, bcid, bmid).await?;
                } else {
                    bot.copy_message(self.s.cid, bcid, bmid).await?;
                }

                self.s.send_menu().await?;

                tokio::task::spawn(async move {
                    let mut count = 0usize;
                    let mut page = 0u32;
                    loop {
                        let Ok(ks) = Karbar::sa_list(&ctx, page).await else {
                            break;
                        };
                        if ks.is_empty() {
                            break;
                        }

                        for mut k in ks {
                            let is_err = if df {
                                bot.forward_message(k.cid(), bcid, bmid)
                                    .await
                                    .is_err()
                            } else {
                                bot.copy_message(k.cid(), bcid, bmid)
                                    .await
                                    .is_err()
                            };

                            if is_err {
                                k.blocked = true;
                                let _ = k.set(&ctx).await;
                            } else {
                                count += 1;
                            }
                        }

                        let m = format!(
                            "🔔 پیام همگانی به {count} کاربر تاکنون ارسال شد ✅"
                        );
                        let _ = bot
                            .send_message(bcid, m)
                            .reply_markup(KeyData::main_menu())
                            .await;
                        tokio::time::sleep(Config::SEND_ALL_SLEEP).await;
                        page += 1;
                    }

                    log::info!("end of loop");

                    let m = format!("پیام همگانی به {count} کاربر ارسال شد ✅");
                    let _ = bot
                        .send_message(bcid, m)
                        .reply_markup(KeyData::main_menu())
                        .await;
                });
            }
        }

        Ok(true)
    }
}
