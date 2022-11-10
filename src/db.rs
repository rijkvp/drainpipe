use crate::{dl::Media, error::Error};
use sqlx::{sqlite::SqliteConnectOptions, FromRow, SqlitePool};
use tracing::log::info;
use std::path::Path;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn load(path: &Path) -> Result<Self, Error> {
        info!("Loading database from {path:?}");
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .create_if_missing(true)
                .filename(path),
        )
        .await?;

        let mut conn = pool.acquire().await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS media (
                source      TEXT PRIMARY KEY NOT NULL,
                id          TEXT NOT NULL,
                path        TEXT NOT NULL,
                title       TEXT,
                description TEXT
            )",
        )
        .execute(&mut conn)
        .await?;

        Ok(Self { pool })
    }

    pub async fn insert(&self, media: &Media) -> Result<(), Error> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query(
            "
            INSERT INTO media (source, id, path, title, description)
            VALUES (?1, ?2, ?3, ?4, ?5)
        ",
        )
        .bind(&media.source)
        .bind(&media.id)
        .bind(&media.path)
        .bind(&media.title)
        .bind(&media.description)
        .execute(&mut conn)
        .await?;

        Ok(())
    }

    pub async fn get_all(&self) -> Result<Vec<Media>, Error> {
        let rows = sqlx::query("SELECT * FROM media")
            .fetch_all(&self.pool)
            .await?;
        let mut res = Vec::new();
        for row in rows {
            res.push(Media::from_row(&row)?);
        }
        Ok(res)
    }

    pub async fn get(&self, link: &str) -> Result<Option<Media>, Error> {
        let row = sqlx::query(
            "
                SELECT * FROM media
                WHERE source=?
            ",
        )
        .bind(link)
        .fetch_optional(&self.pool)
        .await?;
        Ok(if let Some(row) = row {
            Some(Media::from_row(&row)?)
        } else {
            None
        })
    }
}
