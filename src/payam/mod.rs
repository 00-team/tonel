use crate::{
    Ctx, HR, TB,
    config::Config,
    db::{Karbar, Proxy, Settings},
    state::{KeyData, State, Store},
    utils::send_menu,
};
use teloxide::{
    net::Download,
    payloads::{CopyMessageSetters, SendMessageSetters},
    prelude::Requester,
    types::{
        ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message, User,
    },
};

pub struct Payam {
    bot: TB,
    store: Store,
    ctx: Ctx,
    user: User,
    msg: Message,
    karbar: Karbar,
    is_admin: bool,
    state: State,
    cid: ChatId,
    conf: &'static Config,
    settings: Settings,
}

impl Payam {
    pub async fn handle(bot: TB, store: Store, ctx: Ctx, msg: Message) -> HR {
        let Some(user) = &msg.from else { return Ok(()) };
        let karbar = Karbar::init(&ctx, user, "").await?;
        let state = store.get_or_default().await?;
        let is_admin = karbar.is_admin();
        let conf = Config::get();
        let settings = Settings::get(&ctx.db).await;

        let mut payam = Self {
            conf,
            settings,
            cid: msg.chat.id,
            karbar,
            is_admin,
            state,
            ctx,
            store,
            bot,
            user: user.clone(),
            msg,
        };

        if is_admin {
            match state {
                State::AdminProxyAdd => payam.admin_proxy_add().await?,
                State::AdminSetVipMsg => payam.admin_set_vip_msg().await?,
                _ => {}
            }
        }

        Ok(())
    }

    async fn admin_set_vip_msg(&mut self) -> HR {
        let new_msg =
            self.bot.copy_message(self.conf.dev, self.cid, self.msg.id).await?;
        self.settings.vip_msg = Some(new_msg.0 as i64);
        self.settings.set(&self.ctx.db).await?;
        send_menu(&self.bot, &self.store, &self.karbar).await?;
        Ok(())
    }

    async fn admin_proxy_add(&self) -> HR {
        let mut data =
            self.msg.text().map(|v| v.to_string()).unwrap_or_default();

        'd: {
            let Some(doc) = self.msg.document() else { break 'd };
            if doc.file.size > 2 * 1024 * 1024 {
                self.bot.send_message(self.cid, "max file size is 2MB").await?;
                break 'd;
            }
            let m = doc.mime_type.clone();

            if !m.map(|v| v.type_() == "text").unwrap_or_default() {
                self.bot
                    .send_message(self.cid, "only text files are allowed")
                    .await?;
                break 'd;
            }

            let f = self.bot.get_file(doc.file.id.clone()).await?;
            let mut buf = Vec::with_capacity(f.size as usize);
            self.bot.download_file(&f.path, &mut buf).await?;
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
            if px.add(&self.ctx).await.is_ok() {
                added += 1;
            }
        }

        self.bot.send_message(
            self.cid,
            format!(
                "added {added} new proxies\n\nsend other proxies or go to menu"
            ),
        )
        .reply_markup(KeyData::main_menu())
        .await?;

        Ok(())
    }
}
