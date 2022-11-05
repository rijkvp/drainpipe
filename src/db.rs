use crate::{dl::MediaFile, error::Error};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS media (
                source      TEXT PRIMARY KEY NOT NULL,
                id          TEXT NOT NULL,
                path        TEXT NOT NULL,
                title       TEXT,
                description TEXT
            )",
            (),
        )?;
        Ok(Self { conn })
    }

    pub fn insert(&self, media: &MediaFile) -> Result<(), Error> {
        self.conn.execute(
            "
            INSERT INTO media (source, id, path, title, description)
            VALUES (?1, ?2, ?3, ?4, ?5)
        ",
            (
                &media.source,
                &media.id,
                &media.path,
                &media.title,
                &media.description,
            ),
        )?;
        Ok(())
    }

    pub fn get(&self, link: &str) -> Result<Option<MediaFile>, Error> {
        let mut stmt = self.conn.prepare(
            "
                SELECT source, id, path, title, description FROM media
                WHERE source=?
            ",
        )?;
        let media = stmt
            .query_row([link], |row| {
                Ok(MediaFile {
                    source: row.get(0)?,
                    id: row.get(1)?,
                    path: row.get(2)?,
                    title: row.get(3)?,
                    description: row.get(4)?,
                })
            })
            .optional()?;
        Ok(media)
    }
}
