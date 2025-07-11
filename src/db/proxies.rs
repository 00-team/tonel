use crate::{Ctx, book::BookItem, error::AppErr};
use std::fmt::Display;
use teloxide::utils::html::escape;

#[derive(Debug, sqlx::FromRow)]
/// Tonel Proxy
pub struct Proxy {
    pub id: i64,
    pub port: String,
    pub server: String,
    pub secret: String,
    pub up_votes: i64,
    pub dn_votes: i64,
    pub disabled: bool,
}

impl Proxy {
    pub fn up_dn_pct(&self) -> (u8, u8) {
        let ttv = self.up_votes + self.dn_votes;
        let mut upp = 0;
        let mut dnp = 0;
        if self.up_votes > 0 {
            upp = (100 / (ttv / self.up_votes)) as u8;
        }
        if self.dn_votes > 0 {
            dnp = (100 / (ttv / self.dn_votes)) as u8;
        }
        (upp, dnp)
    }

    pub fn url(&self) -> String {
        format!(
            "https://t.me/proxy?server={}&port={}&secret={}",
            self.server, self.port, self.secret
        )
    }

    pub fn from_link(link: &str) -> Option<Self> {
        let mut it = link.splitn(2, "t.me/proxy?");
        it.next()?;
        let spp = it.next()?;

        let mut px = Self {
            id: 0,
            server: String::new(),
            secret: String::new(),
            port: String::new(),
            dn_votes: 0,
            up_votes: 0,
            disabled: false,
        };

        for x in spp.split('&') {
            let mut it = x.splitn(2, '=');
            let Some(key) = it.next() else { continue };
            let Some(val) = it.next() else { continue };
            match key {
                "server" => px.server = val.to_string(),
                "secret" => px.secret = val.to_string(),
                "port" => px.port = val.to_string(),
                _ => {}
            }
        }

        if px.server.is_empty() || px.secret.is_empty() || px.port.is_empty() {
            return None;
        }

        Some(px)
    }

    pub async fn list(ctx: &Ctx, page: u32) -> Result<Vec<Proxy>, AppErr> {
        let offset = page * 32;
        Ok(sqlx::query_as!(
            Proxy,
            "select * from proxies limit 32 offset ?",
            offset
        )
        .fetch_all(&ctx.db)
        .await?)
    }

    pub async fn ch_list(ctx: &Ctx) -> Result<Vec<Proxy>, AppErr> {
        let res = sqlx::query_as!(
            Self,
            "select * from proxies order by RANDOM() limit 4",
        )
        .fetch_all(&ctx.db)
        .await?;

        Ok(res)
    }

    pub async fn count(ctx: &Ctx) -> Result<u32, AppErr> {
        let count = sqlx::query!("select COUNT(1) as count from proxies")
            .fetch_one(&ctx.db)
            .await?;
        Ok(count.count as u32)
    }

    pub async fn add(&mut self, ctx: &Ctx) -> Result<(), AppErr> {
        let res = sqlx::query! {
            "insert into proxies(server, port, secret) values(?,?,?)",
            self.server, self.port, self.secret
        }
        .execute(&ctx.db)
        .await?;
        self.id = res.last_insert_rowid();
        Ok(())
    }

    pub async fn get(ctx: &Ctx, id: i64) -> Result<Self, AppErr> {
        Ok(sqlx::query_as!(Proxy, "select * from proxies where id = ?", id)
            .fetch_one(&ctx.db)
            .await?)
    }

    pub async fn get_good(ctx: &Ctx) -> Option<Self> {
        sqlx::query_as!(
            Proxy,
            "select * from proxies
            where NOT disabled order by random() limit 1"
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn del(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!("delete from proxies where id = ?", id)
            .execute(&ctx.db)
            .await?;
        Ok(())
    }

    pub async fn disabled_toggle(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!(
            "update proxies set disabled = not disabled where id = ?",
            id
        )
        .execute(&ctx.db)
        .await?;

        Ok(())
    }

    pub async fn votes_reset(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!(
            "update proxies set up_votes = 0, dn_votes = 0 where id = ?",
            id
        )
        .execute(&ctx.db)
        .await?;

        sqlx::query!("delete from proxy_votes where proxy = ?", id)
            .execute(&ctx.db)
            .await?;

        Ok(())
    }

    pub async fn vote_get(ctx: &Ctx, karbar: i64, proxy: i64) -> Option<i8> {
        sqlx::query!(
            "select kind from proxy_votes where karbar = ? AND proxy = ?",
            karbar,
            proxy
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
        .map(|v| if v.kind >= 0 { 1 } else { -1 })
    }

    pub async fn vote_add(
        ctx: &Ctx, karbar: i64, proxy: i64, mut kind: i8,
    ) -> Result<(), AppErr> {
        if kind >= 0 {
            kind = 1;
        } else {
            kind = -1;
        }

        sqlx::query!(
            "insert into proxy_votes(kind, karbar, proxy) values(?,?,?)",
            kind,
            karbar,
            proxy
        )
        .execute(&ctx.db)
        .await?;

        let mut px = Self::get(ctx, proxy).await?;

        if kind == 1 {
            px.up_votes += 1;
        } else {
            px.dn_votes += 1;
        }

        if px.up_votes + px.dn_votes > 100 {
            let (_, dnp) = px.up_dn_pct();
            if dnp > 60 {
                px.disabled = true;
            }
        }

        sqlx::query!(
            "update proxies set
            up_votes = ?, dn_votes = ?, disabled = ? where id = ?",
            px.up_votes,
            px.dn_votes,
            px.disabled,
            px.id
        )
        .execute(&ctx.db)
        .await?;

        Ok(())
    }
}

impl Display for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (upp, dnp) = self.up_dn_pct();
        write!(
            f,
            r#"<a href="{}">{}:{}</a> {upp}% ({}) 👍 | {dnp}% ({}) 👎 ({}) {}"#,
            escape(&self.url()),
            self.server,
            self.port,
            self.up_votes,
            self.dn_votes,
            self.up_votes + self.dn_votes,
            if self.disabled { "❌" } else { "" }
        )
    }
}

impl BookItem for Proxy {
    fn id(&self) -> i64 {
        self.id
    }
}
