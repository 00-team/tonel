use crate::config::Config;
use crate::error::{AppErr, err};
use crate::utils::now;
use crate::{Ctx, utils};
use teloxide::types::{ChatId, User, UserId};

use super::Settings;

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
    pub blocked: bool,
    pub last_request: i64,
    pub price_stack: i64,
}

impl Karbar {
    pub const fn cid(&self) -> ChatId {
        ChatId(self.tid)
    }

    pub fn is_admin(&self) -> bool {
        let conf = Config::get();
        conf.admins.contains(&UserId(self.tid as u64))
    }

    pub async fn find_with_tid(ctx: &Ctx, tid: i64) -> Option<Self> {
        sqlx::query_as! {
            Self, "select * from karbars where tid = ?", tid
        }
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn find_with_username(ctx: &Ctx, uname: &str) -> Option<Self> {
        sqlx::query_as! {
            Self, "select * from karbars where username = ?", uname
        }
        .fetch_optional(&ctx.db)
        .await
        .ok()
        .flatten()
    }

    pub async fn init(ctx: &Ctx, user: &User, c: &str) -> Result<Self, AppErr> {
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
            let _ = Self::invited(ctx, c).await;

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
                blocked: false,
                username,
                updated_at,
                created_at: updated_at,
                points: 0,
                last_daily_point_at: 0,
                invite_code: code,
                last_request: 0,
                price_stack: 0,
            });
        };

        if karbar.banned {
            return err!(Banned);
        }

        karbar.username = username;
        karbar.fullname = fullname;
        karbar.updated_at = updated_at;
        karbar.blocked = false;

        karbar.set(ctx).await?;

        Ok(karbar)
    }

    pub async fn set(&self, ctx: &Ctx) -> Result<(), AppErr> {
        sqlx::query! {"update karbars set
            fullname = ?,
            username = ?,
            banned = ?,
            blocked = ?,
            created_at = ?,
            updated_at = ?,
            points = ?,
            last_daily_point_at = ?,
            last_request = ?,
            price_stack = ?
            where tid = ?
        ",
            self.fullname,
            self.username,
            self.banned,
            self.blocked,
            self.created_at,
            self.updated_at,
            self.points,
            self.last_daily_point_at,
            self.last_request,
            self.price_stack,
            self.tid
        }
        .execute(&ctx.db)
        .await?;

        Ok(())
    }

    pub fn calc_cost(&mut self, cost: i64) -> i64 {
        let now = crate::utils::now();
        if self.last_request + Config::PRICE_STACK_RESET < now {
            self.price_stack = 0;
        }

        self.last_request = now;
        self.price_stack += 1;

        let added = match self.price_stack {
            1 => 0.0,
            2 => 0.001,
            3 => 0.03,
            4 => 0.1,
            5 => 0.3,
            6 => 0.6,
            7 => 1.1,
            8 => 1.7,
            9 => 3.0,
            x => x as f64,
        };

        cost + (cost as f64 * added) as i64
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
        log::info!("adding to : {}", karbar.fullname);

        let added = Settings::get(&ctx.db).await.invite_points;

        karbar.points += added;
        karbar.set(ctx).await?;

        Ok(())
    }

    pub async fn sa_list(ctx: &Ctx, page: u32) -> Result<Vec<Self>, AppErr> {
        let offset = page * 100;
        let res = sqlx::query_as!(
            Self,
            "select * from karbars where NOT blocked limit 100 offset ?",
            offset
        )
        .fetch_all(&ctx.db)
        .await?;
        Ok(res)
    }
}

#[derive(Default)]
pub struct KarbarStats {
    pub total: i64,
    pub blocked: i64,
    pub active_5h: i64,
    pub active_7d: i64,
}

impl KarbarStats {
    pub async fn get(ctx: &Ctx) -> Result<Self, AppErr> {
        let now = crate::utils::now();
        let p5h = now - 5 * 3600;
        let p7d = now - 7 * 24 * 3600;

        // let a5h = sqlx::query! {
        //     "select COUNT(1) as count from karbars where updated_at > ?", p5h
        // }
        // .fetch_one(&ctx.db)
        // .await?;
        //
        // let a7d = sqlx::query! {
        //     "select COUNT(1) as count from karbars where updated_at > ?", p7d
        // }
        // .fetch_one(&ctx.db)
        // .await?;

        let count = sqlx::query! {
            "select
                COUNT(1) as total,
                SUM(blocked) as blocked,
                SUM(updated_at > ?) as active_5h,
                SUM(updated_at > ?) as active_7d
            from karbars", p5h, p7d
        }
        .fetch_one(&ctx.db)
        .await?;

        Ok(Self {
            total: count.total,
            blocked: count.blocked.unwrap_or(-1),
            active_5h: count.active_5h.unwrap_or(-1),
            active_7d: count.active_7d.unwrap_or(-1),
        })
    }
}
