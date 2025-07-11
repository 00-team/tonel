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
                self.s.notify("ابتدا نام تبلیغ را ارسال کنید\nفقط برای نمایش به ادمین ها").await?;
                self.s.store.update(State::AdminFlyerAdd).await?;
            }
            KeyData::BookItem(page, id) => {
                let flyer = Flyer::get(&self.s.ctx, id).await?;
                let msg = indoc::formatdoc!(
                    r#"{} 👆👆👆
                    بازدید: {}
                    حداکثر بازدید: {}
                    فعال: {}"#,
                    flyer.label,
                    flyer.views,
                    flyer.max_views,
                    if flyer.disabled { "❌" } else { "✅" },
                );

                let kyb1 = [
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

                let kyb2 = [
                    InlineKeyboardButton::callback(
                        "بازگشت ⬅️",
                        KeyData::BookPagination(page),
                    ),
                    InlineKeyboardButton::callback(
                        "max views 🐝",
                        kd!(ag, Ag::FlyerSetMaxViews(page, flyer.id)),
                    ),
                    KeyData::main_menu_btn(),
                ];

                let (cid, dev) = (self.s.cid, self.s.conf.dev);
                let mid = MessageId(flyer.mid as i32);
                self.s.bot.copy_message(cid, dev, mid).await?;
                self.s
                    .bot
                    .send_message(cid, msg)
                    .reply_markup(InlineKeyboardMarkup::new([kyb1, kyb2]))
                    .await?;
            }
            KeyData::BookPagination(page) => {
                self.admin_flyer_list(page).await?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }
}
