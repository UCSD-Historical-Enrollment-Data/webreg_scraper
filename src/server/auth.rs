use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use uuid::Uuid;

/// A structure representing a simple authentication manager.
pub struct AuthManager {
    /// The SQLite database that is responsible for holding the database information.
    pub db: Mutex<Connection>,
}

impl AuthManager {
    /// Creates a new instance of the `AuthManager`. This will create a new SQLite table\
    /// containing API keys _if_ the table doesn't exist.
    ///
    /// # Returns
    /// The authentication manager.
    pub fn new() -> Self {
        let conn = Connection::open("auth.db").unwrap();
        conn.execute(include_str!("../../sql/init_table.sql"), ())
            .unwrap();

        Self {
            db: Mutex::new(conn),
        }
    }

    /// Generates an API key that can be used to make requests to this server.
    ///
    /// # Returns
    /// A new API key.
    pub fn generate_api_key(&self) -> String {
        let prefix = Uuid::new_v4().to_string();
        let key = Uuid::new_v4().to_string();
        let conn = self.db.lock().unwrap();

        let date_time = Utc::now();
        let expiration_time = date_time + Duration::days(365);
        conn.execute(
            include_str!("../../sql/insert_table.sql"),
            (&prefix, &key, date_time, expiration_time),
        )
        .unwrap();

        format!("{prefix}#{key}")
    }

    /// Checks that the prefix and key that's given is valid.
    ///
    /// # Parameters
    /// - `prefix`: The prefix, used to identify the user.
    /// - `key`: The key.
    ///
    /// # Returns
    /// The check results.
    pub fn check_key(&self, prefix: &str, key: &str) -> AuthCheckResult {
        let conn = self.db.lock().unwrap();
        let mut row = conn
            .prepare(include_str!("../../sql/get_by_prefix.sql"))
            .unwrap();
        let mut res: Vec<_> = row
            .query_map(params![prefix, key], |row| {
                let expiration_time = row.get::<_, DateTime<Utc>>(3).unwrap();
                Ok(expiration_time)
            })
            .unwrap()
            .collect();

        if res.is_empty() {
            return AuthCheckResult::NoPrefixOrKeyFound;
        }

        let elem = res.pop().unwrap();
        let expiration_time = elem.unwrap();
        if expiration_time.timestamp() - Utc::now().timestamp() < 0 {
            return AuthCheckResult::ExpiredKey;
        }

        AuthCheckResult::Valid
    }
}

pub enum AuthCheckResult {
    Valid,
    NoPrefixOrKeyFound,
    ExpiredKey,
}
