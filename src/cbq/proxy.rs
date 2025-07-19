use super::*;

impl Cbq {
    pub async fn admin_proxy_list(&self, page: u32) -> HR {
        let proxies = Proxy::list(&self.s.ctx, page).await?;
        let (total, active) = Proxy::count(&self.s.ctx).await?;
        let bk = Book::new(proxies, page, total / 32);
        let msg = format!(
            "Proxy List Page\npage: {page} | total: {total} | active: {active}\n\n{}",
            &bk.message()
        );

        self.s
            .bot
            .send_message(self.s.cid, msg)
            .parse_mode(ParseMode::Html)
            .reply_markup(bk.keyboard())
            .await?;
        self.s.store.update(State::AdminProxyList).await?;
        self.del_msg().await?;

        Ok(())
    }

    pub async fn handle_admin_proxy(&self) -> Result<bool, AppErr> {
        match self.key {
            KeyData::BookAdd => {
                self.s
                    .bot
                    .send_message(
                        self.s.cid,
                        concat!(
                            "send a proxy links. each link must be on ",
                            "a different line. like:\n\nproxy 1\nproxy 2\n",
                            "proxy 3.\n\n send in a message or a .txt file"
                        ),
                    )
                    .reply_markup(KeyData::main_menu())
                    .await?;
                self.s.store.update(State::AdminProxyAdd).await?;
            }
            KeyData::BookItem(page, id) => {
                let px = Proxy::get(&self.s.ctx, id).await?;
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

                self.s
                    .bot
                    .send_message(self.s.cid, msg)
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
            KeyData::BookDeleteAll => {
                let m = concat!(
                    "آیا از حذف تمامی پروکسی ها اتمینان کامل دارید ❓❓❓\n\n",
                    "این عملیات غیر قابل بازگشت است ⚠⚠⚠"
                );

                let kyb = InlineKeyboardMarkup::new([[
                    KeyData::main_menu_btn(),
                    InlineKeyboardButton::callback(
                        "⭕ حذف همه ⭕",
                        kd!(ag, Ag::ProxyDeleteAllConfirm),
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
