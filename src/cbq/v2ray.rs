use crate::db::V2ray;

use super::*;

impl Cbq {
    pub async fn admin_v2ray_list(&self, page: u32) -> HR {
        let proxies = V2ray::list(&self.s.ctx, page).await?;
        let (total, active) = V2ray::count(&self.s.ctx).await?;
        let bk = Book::new(proxies, page, total / 32);
        let msg = format!(
            "V2ray List Page\npage: {page} | total: {total} | active: {active}\n\n{}",
            &bk.message()
        );

        self.s
            .bot
            .send_message(self.s.cid, msg)
            .parse_mode(ParseMode::Html)
            .reply_markup(bk.keyboard())
            .await?;
        self.s.store.update(State::AdminV2rayList).await?;
        self.del_msg().await?;

        Ok(())
    }

    pub async fn handle_admin_v2ray(&self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::BookAdd => {
                self.s
                    .bot
                    .send_message(
                        self.s.cid,
                        concat!(
                            "send a v2ray links. each link must be on ",
                            "a different line. like:\n\nv2ray 1\nproxy 2\n",
                            "v2ray 3.\n\n send in a message or a .txt file"
                        ),
                    )
                    .reply_markup(KeyData::main_menu())
                    .await?;
                self.s.store.update(State::AdminV2rayAdd).await?;
            }
            KeyData::BookItem(page, id) => {
                let v2 = V2ray::get(&self.s.ctx, id).await?;
                let (upp, dnp) = v2.up_dn_pct();
                let msg = indoc::formatdoc!(
                    r#"
                    <b>V2ray</b>:

                    label: {}
                    link: <code>{}</code>
                    
                    up votes: {upp}% ({}) 👍
                    down votes: {dnp}% ({}) 👎
                    فعال: {}
                "#,
                    v2.label,
                    v2.link,
                    v2.up_votes,
                    v2.dn_votes,
                    if v2.disabled { "❌" } else { "✅" },
                );

                self.s
                    .bot
                    .send_message(self.s.cid, msg)
                    .parse_mode(ParseMode::Html)
                    .reply_markup(InlineKeyboardMarkup::new([
                        vec![
                            InlineKeyboardButton::callback(
                                if v2.disabled {
                                    "فعال کن"
                                } else {
                                    "غیرفعال کن"
                                },
                                kd!(ag, Ag::V2rayDisabledToggle(page, v2.id)),
                            ),
                            InlineKeyboardButton::callback(
                                "ریست کردن رای ها ⚠",
                                kd!(ag, Ag::V2rayVotesReset(page, v2.id)),
                            ),
                            InlineKeyboardButton::callback(
                                "حذف کن ⭕",
                                kd!(ag, Ag::V2rayDel(page, v2.id)),
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
                self.admin_v2ray_list(page).await?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }
}
