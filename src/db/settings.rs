use crate::error::AppErr;
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
/// Tonel Bot Settings
pub struct Settings {
    #[allow(dead_code)]
    id: i64,
    pub invite_points: i64,
    pub star_point_price: i64,
    pub free_point_delay: i64,
    pub free_points: i64,
    pub proxy_cost: i64,
    pub v2ray_cost: i64,
    pub vip_cost: i64,
    pub vip_views: i64,
    pub vip_max_views: i64,
    pub vip_msg: Option<i64>,
    pub donate_msg: Option<i64>,
    pub ch_last_sent: i64,
    pub total_stars: i64,
    pub v2ray_last_auto_update: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: 1,
            invite_points: 100,
            star_point_price: 2,
            free_point_delay: 43200,
            free_points: 100,
            proxy_cost: 100,
            v2ray_cost: 100,
            vip_cost: 200,
            vip_msg: None,
            vip_views: 0,
            vip_max_views: 100,
            donate_msg: None,
            ch_last_sent: 0,
            total_stars: 0,
            v2ray_last_auto_update: 0,
        }
    }
}

impl Settings {
    pub async fn get(pool: &SqlitePool) -> Self {
        let Ok(Some(settings)) = sqlx::query_as! {
            Settings, "select * from settings where id = 1"
        }
        .fetch_optional(pool)
        .await
        else {
            let _ = sqlx::query!("insert into settings(id) values(1)")
                .execute(pool)
                .await;
            return Self::default();
        };
        settings
    }

    pub async fn set(&mut self, pool: &SqlitePool) -> Result<(), AppErr> {
        if self.vip_max_views > -1 && self.vip_views > self.vip_max_views {
            self.vip_msg = None;
            self.vip_views = 0;
        }

        let old = sqlx::query_as! {
            Settings, "select * from settings where id = 1"
        }
        .fetch_optional(pool)
        .await?;

        if old.is_none() {
            sqlx::query!("insert into settings(id) values(1)")
                .execute(pool)
                .await?;
            return Ok(());
        }

        sqlx::query! {"
            update settings set 
            invite_points = ?,
            free_points = ?,
            proxy_cost = ?,
            v2ray_cost = ?,
            vip_cost = ?,
            vip_msg = ?,
            vip_views = ?,
            vip_max_views = ?,
            donate_msg = ?,
            ch_last_sent = ?,
            free_point_delay = ?,
            total_stars = ?,
            star_point_price = ?,
            v2ray_last_auto_update = ?
            where id = 1
        ",
            self.invite_points,
            self.free_points,
            self.proxy_cost,
            self.v2ray_cost,
            self.vip_cost,
            self.vip_msg,
            self.vip_views,
            self.vip_max_views,
            self.donate_msg,
            self.ch_last_sent,
            self.free_point_delay,
            self.total_stars,
            self.star_point_price,
            self.v2ray_last_auto_update
        }
        .execute(pool)
        .await?;

        Ok(())
    }
}
