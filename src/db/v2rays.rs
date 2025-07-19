use crate::{Ctx, book::BookItem, error::AppErr};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, sqlx::FromRow)]
/// Tonel V2ray
pub struct V2ray {
    pub id: i64,
    pub label: String,
    pub link: String,
    pub up_votes: i64,
    pub dn_votes: i64,
    pub disabled: bool,
}

impl V2ray {
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

    pub fn from_link(link: &str) -> Option<Self> {
        let link = link.trim();
        if link.is_empty() {
            return None;
        }
        let label = if let Ok(url) = reqwest::Url::from_str(link) {
            url.host_str().unwrap_or("<no host>").to_string()
        } else {
            let mut out = String::with_capacity(32);
            for ch in link.chars().take(32) {
                out.push(ch);
            }
            out
        };

        let v2 = Self {
            id: 0,
            label,
            link: link.to_string(),
            dn_votes: 0,
            up_votes: 0,
            disabled: false,
        };

        Some(v2)
    }

    pub async fn list(ctx: &Ctx, page: u32) -> Result<Vec<Self>, AppErr> {
        let offset = page * 32;
        Ok(sqlx::query_as!(
            Self,
            "select * from v2rays limit 32 offset ?",
            offset
        )
        .fetch_all(&ctx.db)
        .await?)
    }

    pub async fn count(ctx: &Ctx) -> Result<(u32, u32), AppErr> {
        let count = sqlx::query!(
            "select
                COUNT(1) as total,
                SUM(NOT disabled) as active
            from v2rays"
        )
        .fetch_one(&ctx.db)
        .await?;

        Ok((count.total as u32, count.active.unwrap_or_default() as u32))
    }

    pub async fn add(&mut self, ctx: &Ctx) -> Result<(), AppErr> {
        let res = sqlx::query! {
            "insert into v2rays(label, link) values(?,?)",
            self.label, self.link
        }
        .execute(&ctx.db)
        .await?;
        self.id = res.last_insert_rowid();
        Ok(())
    }

    pub async fn get(ctx: &Ctx, id: i64) -> Result<Self, AppErr> {
        Ok(sqlx::query_as!(Self, "select * from v2rays where id = ?", id)
            .fetch_one(&ctx.db)
            .await?)
    }

    pub async fn get_good(ctx: &Ctx) -> Option<Self> {
        sqlx::query_as!(
            Self,
            "select * from v2rays
            where NOT disabled order by random() limit 1"
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn del(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!("delete from v2rays where id = ?", id)
            .execute(&ctx.db)
            .await?;
        Ok(())
    }

    pub async fn del_all(ctx: &Ctx) -> Result<(), AppErr> {
        sqlx::query!("delete from v2rays").execute(&ctx.db).await?;
        Ok(())
    }

    pub async fn disabled_toggle(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!(
            "update v2rays set disabled = not disabled where id = ?",
            id
        )
        .execute(&ctx.db)
        .await?;

        Ok(())
    }

    pub async fn votes_reset(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!(
            "update v2rays set up_votes = 0, dn_votes = 0 where id = ?",
            id
        )
        .execute(&ctx.db)
        .await?;

        sqlx::query!("delete from v2rays_votes where v2ray = ?", id)
            .execute(&ctx.db)
            .await?;

        Ok(())
    }

    pub async fn vote_get(ctx: &Ctx, karbar: i64, v2: i64) -> Option<i8> {
        sqlx::query!(
            "select kind from v2rays_votes where karbar = ? AND v2ray = ?",
            karbar,
            v2
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
        .map(|v| if v.kind >= 0 { 1 } else { -1 })
    }

    pub async fn vote_add(
        ctx: &Ctx, karbar: i64, v2: i64, mut kind: i8,
    ) -> Result<(), AppErr> {
        if kind >= 0 {
            kind = 1;
        } else {
            kind = -1;
        }

        sqlx::query!(
            "insert into v2rays_votes(kind, karbar, v2ray) values(?,?,?)",
            kind,
            karbar,
            v2
        )
        .execute(&ctx.db)
        .await?;

        let mut v2 = Self::get(ctx, v2).await?;

        if kind == 1 {
            v2.up_votes += 1;
        } else {
            v2.dn_votes += 1;
        }

        if v2.up_votes + v2.dn_votes > 25 {
            let (_, dnp) = v2.up_dn_pct();
            if dnp > 60 {
                return Self::del(ctx, v2.id).await;
                // v2.disabled = true;
            }
        }

        sqlx::query!(
            "update v2rays set
            up_votes = ?, dn_votes = ?, disabled = ? where id = ?",
            v2.up_votes,
            v2.dn_votes,
            v2.disabled,
            v2.id
        )
        .execute(&ctx.db)
        .await?;

        Ok(())
    }
}

impl Display for V2ray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (upp, dnp) = self.up_dn_pct();
        write!(
            f,
            r#"{} {upp}% ({}) 👍 | {dnp}% ({}) 👎 ({}) {}"#,
            self.label,
            self.up_votes,
            self.dn_votes,
            self.up_votes + self.dn_votes,
            if self.disabled { "❌" } else { "" }
        )
    }
}

impl BookItem for V2ray {
    fn id(&self) -> i64 {
        self.id
    }
}
