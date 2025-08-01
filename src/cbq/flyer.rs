use teloxide::sugar::request::RequestLinkPreviewExt;

use super::*;

impl super::Cbq {
    pub async fn admin_flyer_list(&self, page: u32) -> HR {
        let flyers = Flyer::list(&self.s.ctx, page).await?;
        let count = Flyer::count(&self.s.ctx).await?;
        let bk = Book::new(flyers, page, count / 32);
        let msg = format!(
            "لیست تبلیغات\npage: {page} | total: {count}\n\n{}",
            &bk.message()
        );

        self.s
            .bot
            .send_message(self.s.cid, msg)
            .parse_mode(ParseMode::Html)
            .reply_markup(bk.keyboard())
            .await?;
        self.s.store.update(State::AdminFlyerList).await?;
        self.del_msg().await?;

        Ok(())
    }

    pub async fn handle_admin_flyer(&self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::BookAdd => {
                let m = concat!(
                    "ابتدا نام تبلیغ را ارسال کنید\n\n",
                    "این نام برای دکمه لینک تبلیغ استفاده می شود"
                );
                self.s.store.update(State::AdminFlyerAdd).await?;
                self.s.notify(m).await?;
            }
            KeyData::BookItem(page, id) => {
                let flyer = Flyer::get(&self.s.ctx, id).await?;
                let msg = indoc::formatdoc!(
                    r#"{} 👆👆👆
                    بازدید: {}
                    حداکثر بازدید: {}
                    فعال: {}
                    link: {}"#,
                    flyer.label,
                    flyer.views,
                    flyer.max_views,
                    if flyer.disabled { "❌" } else { "✅" },
                    flyer.link.as_deref().unwrap_or("---")
                );

                let kyb1 = vec![
                    InlineKeyboardButton::callback(
                        if flyer.disabled {
                            "فعال کن"
                        } else {
                            "غیرفعال کن"
                        },
                        kd!(ag, Ag::FlyerDisabledToggle(page, flyer.id)),
                    ),
                    InlineKeyboardButton::callback(
                        "reset views ⚠",
                        kd!(ag, Ag::FlyerViewsReset(page, flyer.id)),
                    ),
                    InlineKeyboardButton::callback(
                        "حذف کن ⭕",
                        kd!(ag, Ag::FlyerDel(page, flyer.id)),
                    ),
                ];

                let kyb2 = vec![
                    InlineKeyboardButton::callback(
                        "max views 🐝",
                        kd!(ag, Ag::FlyerSetMaxViews(page, flyer.id)),
                    ),
                    InlineKeyboardButton::callback(
                        "عنوان 🏷️",
                        kd!(ag, Ag::FlyerSetLabel(page, flyer.id)),
                    ),
                    InlineKeyboardButton::callback(
                        "ثبت لینک 🔗",
                        kd!(ag, Ag::FlyerSetLink(page, flyer.id)),
                    ),
                ];

                let mut kyb3 = vec![
                    InlineKeyboardButton::callback(
                        "بازگشت ⬅️",
                        KeyData::BookPagination(page),
                    ),
                    KeyData::main_menu_btn(),
                ];

                if flyer.link.is_some() {
                    kyb3.push(InlineKeyboardButton::callback(
                        "حذف لینک ⭕",
                        kd!(ag, Ag::FlyerDelLink(page, flyer.id)),
                    ));
                }

                let kb = InlineKeyboardMarkup::new([kyb1, kyb2, kyb3]);

                let (cid, dev) = (self.s.cid, self.s.conf.dev);
                let mid = MessageId(flyer.mid as i32);
                self.s.bot.copy_message(cid, dev, mid).await?;
                self.s
                    .bot
                    .send_message(cid, msg)
                    .reply_markup(kb)
                    .disable_link_preview(true)
                    .await?;
            }
            KeyData::BookPagination(page) => {
                self.admin_flyer_list(page).await?;
            }
            KeyData::BookDeleteAll => {
                let m = concat!(
                    "آیا از حذف تمامی تبلیغات اتمینان کامل دارید ❓❓❓\n\n",
                    "این عملیات غیر قابل بازگشت است ⚠⚠⚠"
                );

                let kyb = InlineKeyboardMarkup::new([[
                    KeyData::main_menu_btn(),
                    InlineKeyboardButton::callback(
                        "⭕ حذف همه ⭕",
                        kd!(ag, Ag::FlyerDeleteAllConfirm),
                    ),
                    KeyData::main_menu_btn(),
                ]]);

                let cid = self.s.cid;
                self.s.bot.send_message(cid, m).reply_markup(kyb).await?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }
}
