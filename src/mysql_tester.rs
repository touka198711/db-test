

use std::{thread, path::Path};

use sqlx::{Connection, Executor, migrate::Migrator, MySqlConnection};
use uuid::Uuid;

use crate::MySqlTest;


impl MySqlTest {
    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!("mysql://{}@{}:{}", self.username, self.host, self.port)
        } else {
            format!("mysql://{}:{}@{}:{}", self.username, self.password, self.host, self.port)
        }
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }
}

impl MySqlTest {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        migration_path: impl Into<String>,
    ) -> Self {
        let uuid = Uuid::new_v4();
        let dbname = format!("test_{}", uuid);
        
        let mysql_test = MySqlTest {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            dbname: dbname.clone(),
        };

        let url = mysql_test.server_url();
        let migration_path: String = migration_path.into();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = MySqlConnection::connect(&url).await.unwrap();
                conn.execute(format!(r#"CREATE DATABASE `{}`;"#, dbname).as_str())
                    .await
                    .expect("failed to create database");

                let url = format!("{}/{}", url, dbname);
                let mut conn = MySqlConnection::connect(&url).await.unwrap();
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

        mysql_test
    }
}

impl Drop for MySqlTest {
    fn drop(&mut self) {
        let url = self.server_url();
        let dbname = self.dbname.clone();

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = MySqlConnection::connect(&url).await.unwrap();
                // terminate existing connections
                // sqlx::query(&format!(
                //     r#"
                //     SELECT pg_terminate_backend(pid) FROM pg_stat_activity 
                //     WHERE pid <> pg_backend_pid() AND datname = '{}'"#,
                //     dbname
                // ))
                // .execute(&mut conn)
                // .await
                // .unwrap();

                conn.execute(format!(r#"DROP DATABASE `{}`;"#, dbname).as_str())
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
    use sqlx::{mysql::MySqlPoolOptions, Row};

    use super::*;

    #[tokio::test]
    async fn mysql_test_should_work() {
        let mysql_test = MySqlTest::new("localhost", 3306, "root", "root", "./migrations");
        let pool = MySqlPoolOptions::new()
            .connect(&mysql_test.url())
            .await
            .expect("failed to create dataabse");
        let id = sqlx::query("INSERT INTO test (title) VALUES ('hello')")
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_id();
        
        assert_eq!(id, 1);

        let row = sqlx::query("SELECT * FROM test WHERE id = 1")
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
