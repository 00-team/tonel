use std::fmt::Display;

use teloxide::utils::html::escape;

use crate::{Ctx, book::BookItem, error::AppErr};

pub struct Flyer {
    pub id: i64,
    pub label: String,
    pub link: Option<String>,
    pub mid: i64,
    pub views: i64,
    pub max_views: i64,
    pub disabled: bool,
}

impl Default for Flyer {
    fn default() -> Self {
        Self {
            id: 0,
            label: String::new(),
            link: None,
            mid: 0,
            views: 0,
            max_views: -1,
            disabled: false,
        }
    }
}

impl Flyer {
    pub fn new(label: String, mid: i64) -> Self {
        Self { label, mid, ..Default::default() }
    }

    pub async fn list(ctx: &Ctx, page: u32) -> Result<Vec<Self>, AppErr> {
        let offset = page * 32;
        let res = sqlx::query_as!(
            Self,
            "select * from flyers limit 32 offset ?",
            offset
        )
        .fetch_all(&ctx.db)
        .await?;
        Ok(res)
    }

    pub async fn count(ctx: &Ctx) -> Result<u32, AppErr> {
        let count = sqlx::query!("select COUNT(1) as count from flyers")
            .fetch_one(&ctx.db)
            .await?;
        Ok(count.count as u32)
    }

    pub async fn add(&mut self, ctx: &Ctx) -> Result<(), AppErr> {
        let res = sqlx::query! {
            "insert into flyers(label, mid) values(?,?)",
            self.label, self.mid
        }
        .execute(&ctx.db)
        .await?;
        self.id = res.last_insert_rowid();
        Ok(())
    }

    pub async fn get(ctx: &Ctx, id: i64) -> Result<Self, AppErr> {
        let rs = sqlx::query_as!(Self, "select * from flyers where id = ?", id)
            .fetch_one(&ctx.db)
            .await?;

        Ok(rs)
    }

    pub async fn get_good(ctx: &Ctx) -> Option<Self> {
        sqlx::query_as!(
            Self,
            "select * from flyers
            where NOT (disabled OR (max_views > -1 AND views >= max_views))
            order by random() limit 1"
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn get_good_link(ctx: &Ctx) -> Option<Self> {
        sqlx::query_as!(
            Self,
            "select * from flyers
            where link is not NULL AND
            NOT (disabled OR (max_views > -1 AND views >= max_views))
            order by random() limit 1"
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn del(ctx: &Ctx, id: i64) -> Result<(), AppErr> {
        sqlx::query!("delete from flyers where id = ?", id)
            .execute(&ctx.db)
            .await?;
        Ok(())
    }

    pub async fn set(&mut self, ctx: &Ctx) -> Result<(), AppErr> {
        if self.max_views > -1 && self.views >= self.max_views {
            self.disabled = true;
        }

        sqlx::query!(
            "update flyers set
            disabled = ?,
            views = ?,
            link = ?,
            max_views = ?
            where id = ?",
            self.disabled,
            self.views,
            self.link,
            self.max_views,
            self.id
        )
        .execute(&ctx.db)
        .await?;

        Ok(())
    }
}

impl Display for Flyer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"<b>{}</b> {}/{} {}"#,
            escape(&self.label),
            self.views,
            self.max_views,
            if self.disabled { "❌" } else { "" }
        )
    }
}

impl BookItem for Flyer {
    fn id(&self) -> i64 {
        self.id
    }
}
