mod pg_tester;
mod mysql_tester;

pub struct PgTest {
    host: String,
    port: u16,
    username: String,
    password: String,
    dbname: String,
}

pub struct MySqlTest {
    host: String,
    port: u16,
    username: String,
    password: String,
    dbname: String,
}