use sqlx::SqlitePool;
use crate::error::AppErr;

#[derive(Debug, Clone)]
/// Tonel Bot Settings
pub struct Settings {
    id: i64,
    pub invite_points: i64,
    pub daily_points: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self { id: 1, invite_points: 100, daily_points: 100 }
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

    pub async fn set(&self, pool: &SqlitePool) -> Result<(), AppErr> {
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
            daily_points = ?
            where id = 1
        ",
            self.invite_points,
            self.daily_points
        }
        .execute(pool)
        .await?;

        Ok(())
    }
}
