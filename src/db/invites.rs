use crate::{Ctx, error::AppErr};

pub struct InviteLink {
    pub link: String,
    pub karbar: i64,
    pub count: i64,
}

impl InviteLink {
    pub async fn invited(ctx: &Ctx, link: &str) -> Result<(), AppErr> {
        let Some(inv) = Self::get(ctx, link).await else { return Ok(()) };

        let added = { ctx.settings.lock().await.invite_points };

        sqlx::query! {
            "update karbars set points = points + ? where tid = ?",
            added, inv.karbar
        }
        .execute(&ctx.db)
        .await?;

        sqlx::query! {
            "update invite_links set count = count + 1 where link = ?",
            link
        }
        .execute(&ctx.db)
        .await?;

        Ok(())
    }

    pub async fn get(ctx: &Ctx, link: &str) -> Option<InviteLink> {
        if link.is_empty() {
            return None;
        }

        sqlx::query_as!(
            InviteLink,
            "select * from invite_links where link = ?",
            link
        )
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn add(&self, ctx: &Ctx) -> Result<(), AppErr> {
        sqlx::query! {
            "insert into invite_links(link, karbar) values(?, ?)",
            self.link, self.karbar
        }
        .execute(&ctx.db)
        .await?;

        Ok(())
    }
}
