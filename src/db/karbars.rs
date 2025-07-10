use crate::config::Config;
use crate::error::{AppErr, err};
use crate::utils::now;
use crate::{Ctx, utils};
use teloxide::types::{ChatId, User, UserId};

#[derive(Debug, sqlx::FromRow, Clone)]
pub struct Karbar {
    pub tid: i64,
    pub fullname: String,
    // without leading @
    pub username: Option<String>,
    pub banned: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub points: i64,
    pub last_daily_point_at: i64,
    pub invite_code: String,
}

impl Karbar {
    pub const fn cid(&self) -> ChatId {
        ChatId(self.tid)
    }

    pub fn is_admin(&self) -> bool {
        let conf = Config::get();
        conf.admins.contains(&UserId(self.tid as u64))
    }

    pub async fn init(ctx: &Ctx, user: &User, r: &str) -> Result<Self, AppErr> {
        let tid = user.id.0 as i64;
        let fullname = user.full_name();
        let username = user.username.clone();
        let updated_at = now();

        let karbar = sqlx::query_as! {
            Karbar, "select * from karbars where tid = ?", tid
        }
        .fetch_optional(&ctx.db)
        .await?;

        let Some(mut karbar) = karbar else {
            let _ = Self::invited(ctx, r).await;

            let code = loop {
                let code = utils::random_code();
                let r = sqlx::query!(
                    "select tid from karbars where invite_code = ?",
                    code
                )
                .fetch_optional(&ctx.db)
                .await?;
                if r.is_none() {
                    break code;
                }
            };

            sqlx::query! {"
            insert into karbars (
                tid,
                fullname,
                username,
                created_at,
                updated_at,
                invite_code
            ) values(?,?,?,?,?,?)",
                tid,
                fullname,
                username,
                updated_at,
                updated_at,
                code
            }
            .execute(&ctx.db)
            .await?;

            return Ok(Self {
                tid,
                fullname,
                banned: false,
                username,
                updated_at,
                created_at: updated_at,
                points: 0,
                last_daily_point_at: 0,
                invite_code: code,
            });
        };

        if karbar.banned {
            return err!(Banned);
        }

        karbar.username = username;
        karbar.fullname = fullname;
        karbar.updated_at = updated_at;

        karbar.set(ctx).await?;

        Ok(karbar)
    }

    pub async fn set(&self, ctx: &Ctx) -> Result<(), AppErr> {
        sqlx::query! {"update karbars set
            fullname = ?,
            username = ?,
            banned = ?,
            created_at = ?,
            updated_at = ?,
            points = ?,
            last_daily_point_at = ?
            where tid = ?
        ",
            self.fullname,
            self.username,
            self.banned,
            self.created_at,
            self.updated_at,
            self.points,
            self.last_daily_point_at,
            self.tid
        }
        .execute(&ctx.db)
        .await?;

        Ok(())
    }

    pub async fn invited(ctx: &Ctx, code: &str) -> Result<(), AppErr> {
        if code.is_empty() {
            return Ok(());
        }

        let karbar = sqlx::query_as!(
            Karbar,
            "select * from karbars where invite_code = ?",
            code
        )
        .fetch_optional(&ctx.db)
        .await?;

        let Some(mut karbar) = karbar else { return Ok(()) };

        let added = { ctx.settings.lock().await.invite_points };
        karbar.points += added;
        karbar.set(ctx).await?;

        Ok(())
    }
}
