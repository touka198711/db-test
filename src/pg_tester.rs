use std::{thread, path::Path};

use sqlx::{PgConnection, Connection, Executor, migrate::Migrator};
use uuid::Uuid;

use crate::PgTest;


impl PgTest {
    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!("postgres://{}@{}:{}", self.username, self.host, self.port)
        } else {
            format!("postgres://{}:{}@{}:{}", self.username, self.password, self.host, self.port)
        }
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }
}

impl PgTest {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        migration_path: impl Into<String>,
    ) -> Self {
        let uuid = Uuid::new_v4();
        let dbname = format!("test_{}", uuid);
        
        let pg_test = PgTest {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            dbname: dbname.clone(),
        };

        let url = pg_test.server_url();
        let migration_path: String = migration_path.into();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = PgConnection::connect(&url).await.unwrap();
                conn.execute(format!(r#"CREATE DATABASE "{}";"#, dbname).as_str())
                    .await
                    .expect("failed to create database");

                let url = format!("{}/{}", url, dbname);
                let mut conn = PgConnection::connect(&url).await.unwrap();
                Migrator::new(Path::new(migration_path.as_str()))
                    .await
                    .expect("failed to migrate")
                    .run(&mut conn)
                    .await
                    .expect("failed to migrate database");
            });
        })
        .join()
        .expect("failed to create database");

        pg_test
    }
}

impl Drop for PgTest {
    fn drop(&mut self) {
        let url = self.server_url();
        let dbname = self.dbname.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = PgConnection::connect(&url).await.unwrap();
                // terminate existing connections
                sqlx::query(&format!(
                    r#"
                    SELECT pg_terminate_backend(pid) FROM pg_stat_activity 
                    WHERE pid <> pg_backend_pid() AND datname = '{}'"#,
                    dbname
                ))
                .execute(&mut conn)
                .await
                .unwrap();

                conn.execute(format!(r#"DROP DATABASE "{}";"#, dbname).as_str())
                    .await
                    .expect("Error while querying the drop database");
            })
        })
        .join()
        .expect("failed to drop database");
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{postgres::PgPoolOptions, Row};

    use super::*;

    #[tokio::test]
    async fn pg_test_should_work() {
        let pg_test = PgTest::new("localhost", 5432, "Touka", "root", "./migrations");
        let pool = PgPoolOptions::new()
            .connect(&pg_test.url())
            .await
            .expect("failed to create dataabse");
        let row = sqlx::query("INSERT INTO test (title) VALUES ('hello') RETURNING id, title, complex")
            .fetch_one(&pool)
            .await
            .unwrap();
        let id: i32 = row.get("id");
        let title: String = row.get("title");
        let complex: bool = row.get("complex");
        assert_eq!(id, 1);
        assert_eq!(&title, "hello");
        assert_eq!(complex, false);
    }
}
