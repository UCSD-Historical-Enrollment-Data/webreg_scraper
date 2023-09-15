use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use uuid::Uuid;

const EXP_AT_COLUMN: &str = "expires_at";
const PREFIX_COLUMN: &str = "prefix";
const TOKEN_COLUMN: &str = "token";
const CREATED_AT_COLUMN: &str = "created_at";
const DESCRIPTION_COLUMN: &str = "description";

/// A structure representing a simple authentication manager.
pub struct AuthManager {
    /// The SQLite database that is responsible for holding the database information.
    pub db: Mutex<Connection>,
}

impl AuthManager {
    /// Creates a new instance of the `AuthManager`. This will create a new SQLite table
    /// containing API keys _if_ the table doesn't exist.
    ///
    /// # Parameters
    /// - `db_name`: The name of the database file.
    ///
    /// # Returns
    /// The authentication manager.
    pub fn new(db_name: &str) -> Self {
        let conn = Connection::open(db_name).unwrap();
        conn.execute(include_str!("../../../sql/init_table.sql"), ())
            .unwrap();

        Self {
            db: Mutex::new(conn),
        }
    }

    /// Generates an API key that can be used to make requests to this server.
    ///
    /// # Parameters
    /// - `desc`: A description for this API key, if any.
    ///
    /// # Returns
    /// A new API key.
    pub fn generate_api_key(&self, desc: Option<&str>) -> String {
        let prefix = Uuid::new_v4().to_string();
        let key = Uuid::new_v4().to_string();
        let conn = self.db.lock().unwrap();

        let date_time = Utc::now();
        let expiration_time = date_time + Duration::days(365);
        conn.execute(
            include_str!("../../../sql/insert_table.sql"),
            params![&prefix, &key, date_time, expiration_time, desc],
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
        let mut stmt = conn
            .prepare(include_str!("../../../sql/get_by_prefix.sql"))
            .unwrap();
        let mut res: Vec<_> = stmt
            .query_map(params![prefix, key], |row| {
                Ok(row.get::<_, DateTime<Utc>>(EXP_AT_COLUMN).unwrap())
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

    /// Attempts to delete a prefix and associated key from the authentication
    /// database, rendering it unable to be used when making requests to the API.
    ///
    /// # Parameters
    /// - `prefix`: The prefix, used to identify the user.
    ///
    /// # Returns
    /// `true` if deletion was successful, and `false` otherwise.
    pub fn delete_by_prefix(&self, prefix: &str) -> bool {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn
            .prepare(include_str!("../../../sql/delete_by_prefix.sql"))
            .unwrap();

        match stmt.execute(params![prefix]) {
            Ok(n) if n > 0 => true,
            _ => false,
        }
    }

    /// Edits the description associated with a prefix.
    ///
    /// # Parameters
    /// - `prefix`: The prefix to modify.
    /// - `desc`: A description to be associated with this prefix, if any.
    ///
    /// # Returns
    /// `true` if modification was successful, and `false` otherwise.
    pub fn edit_description_by_prefix(&self, prefix: &str, desc: Option<&str>) -> bool {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn
            .prepare(include_str!("../../../sql/edit_desc_by_prefix.sql"))
            .unwrap();

        match stmt.execute(params![desc, prefix]) {
            Ok(n) if n > 0 => true,
            _ => false,
        }
    }

    /// Gets all prefixes currently in this database.
    ///
    /// # Returns
    /// A list of all prefixes.
    pub fn get_all_prefixes(&self) -> Vec<String> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn
            .prepare(include_str!("../../../sql/get_all_entries.sql"))
            .unwrap();

        stmt.query_map((), |row| Ok(row.get::<_, String>(PREFIX_COLUMN).unwrap()))
            .unwrap()
            .map(|data| data.unwrap())
            .collect()
    }

    /// Gets all entries currently in this database.
    ///
    /// # Returns
    /// A list of all entries.
    pub fn get_all_entries(&self) -> Vec<KeyEntry> {
        let conn = self.db.lock().unwrap();
        let mut stmt = conn
            .prepare(include_str!("../../../sql/get_all_entries.sql"))
            .unwrap();

        stmt.query_map((), |row| {
            Ok(KeyEntry {
                prefix: row.get::<_, String>(PREFIX_COLUMN).unwrap(),
                token: row.get::<_, String>(TOKEN_COLUMN).unwrap(),
                created_at: row.get::<_, DateTime<Utc>>(CREATED_AT_COLUMN).unwrap(),
                expires_at: row.get::<_, DateTime<Utc>>(EXP_AT_COLUMN).unwrap(),
                description: row.get::<_, Option<String>>(DESCRIPTION_COLUMN).unwrap(),
            })
        })
        .unwrap()
        .map(|data| data.unwrap())
        .collect()
    }
}

/// An enum representing the result of checking for the prefix and key.
#[derive(Eq, PartialEq, Debug)]
pub enum AuthCheckResult {
    /// Whether the prefix exists and the associated key is valid.
    Valid,
    /// Whether the prefix does not exist, or the key is not found.
    NoPrefixOrKeyFound,
    /// Whether the key has expired.
    ExpiredKey,
}

/// Represents an entry in the database.
pub struct KeyEntry {
    /// The prefix for this API key.
    pub prefix: String,
    /// The token for this API key.
    pub token: String,
    /// When this API key was created.
    pub created_at: DateTime<Utc>,
    /// When this API key will expire.
    pub expires_at: DateTime<Utc>,
    /// Any description for this key.
    pub description: Option<String>,
}
