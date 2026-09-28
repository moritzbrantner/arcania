use std::error::Error;
use std::fmt;

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use rand::Rng;
use rand::rand_core::UnwrapErr;
use rand::rngs::SysRng;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::match_session::HeroType;
use crate::preferences;

mod handles;
mod passwords;
mod sessions;

pub const EXPERIENCED_LOCAL_EMAIL: &str = "experienced@local.dev";
pub const EXPERIENCED_LOCAL_PASSWORD: &str = "experienced";
pub const EXPERIENCED_LOCAL_XP: i64 = 20_000;
const EXPERIENCED_LOCAL_USER_ID: i64 = 10_000;

#[derive(Clone, Debug)]
pub struct AccountProfile {
    pub id: i64,
    pub public_handle: String,
    pub email: String,
    pub display_name: String,
    pub avatar: GeneratedAvatar,
    pub preferred_hero_type: HeroType,
    pub board_visual_mode: BoardVisualMode,
    pub total_xp: i64,
}

#[derive(Clone, Debug)]
pub struct GeneratedAvatar {
    pub symbol: String,
    pub color: String,
}

#[derive(Clone, Debug)]
pub struct PublicAccountProfile {
    pub id: i64,
    pub public_handle: String,
    pub display_name: String,
    pub avatar: GeneratedAvatar,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum BoardVisualMode {
    #[serde(rename = "2d")]
    TwoD,
    #[serde(rename = "3d")]
    ThreeD,
}

#[derive(Clone, Debug)]
pub struct CreatedAuthSession {
    pub token: String,
    pub profile: AccountProfile,
}

pub struct IdentityModule<'a> {
    connection: &'a mut Connection,
}

type LoginProfileRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
);

#[derive(Debug)]
pub enum IdentityError {
    Sqlite(rusqlite::Error),
    PasswordHash(argon2::password_hash::Error),
    Validation(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "could not access account database: {error}"),
            Self::PasswordHash(error) => {
                write!(f, "could not process password credentials: {error}")
            }
            Self::Validation(message) => write!(f, "{message}"),
        }
    }
}

impl Error for IdentityError {}

impl From<rusqlite::Error> for IdentityError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<argon2::password_hash::Error> for IdentityError {
    fn from(error: argon2::password_hash::Error) -> Self {
        Self::PasswordHash(error)
    }
}

impl<'a> IdentityModule<'a> {
    pub fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    pub fn register(
        &mut self,
        email: &str,
        normalized_email: &str,
        password: &str,
    ) -> Result<Option<CreatedAuthSession>, IdentityError> {
        let password_hash = hash_password(password)?;
        let avatar = generated_avatar_for_email(normalized_email);
        let inserted = self.connection.execute(
            "
            INSERT OR IGNORE INTO users (
                email,
                email_normalized,
                password_hash,
                display_name,
                avatar_symbol,
                avatar_color,
                preferred_hero_type,
                board_visual_mode,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, unixepoch())
            ",
            params![
                email,
                normalized_email,
                password_hash,
                default_display_name(email),
                avatar.symbol,
                avatar.color,
                hero_type_to_db(HeroType::default()),
                board_visual_mode_to_db(BoardVisualMode::ThreeD)
            ],
        )?;

        if inserted == 0 {
            return Ok(None);
        }

        let user_id = self.connection.last_insert_rowid();
        let display_name = default_display_name(email);
        let public_handle = unique_public_handle(self.connection, user_id, &display_name)?;
        self.connection.execute(
            "UPDATE users SET public_handle = ?2 WHERE id = ?1",
            params![user_id, public_handle],
        )?;

        let profile = self
            .load_profile_by_normalized_email(normalized_email)?
            .expect("newly inserted account should load");
        self.create_session(profile).map(Some)
    }

    pub fn login(
        &mut self,
        normalized_email: &str,
        password: &str,
    ) -> Result<Option<CreatedAuthSession>, IdentityError> {
        let row: Option<LoginProfileRow> = self
            .connection
            .query_row(
                "
                SELECT id, public_handle, email, password_hash, display_name, avatar_symbol, avatar_color, preferred_hero_type, board_visual_mode, total_xp
                FROM users
                WHERE email_normalized = ?1
                ",
                params![normalized_email],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ))
                },
            )
            .optional()?;

        let Some((
            id,
            public_handle,
            email,
            password_hash,
            display_name,
            avatar_symbol,
            avatar_color,
            preferred_hero_type,
            board_visual_mode,
            total_xp,
        )) = row
        else {
            return Ok(None);
        };

        if !password_matches(password, &password_hash) {
            return Ok(None);
        }

        self.create_session(AccountProfile {
            id,
            public_handle,
            email,
            display_name,
            avatar: GeneratedAvatar {
                symbol: avatar_symbol,
                color: avatar_color,
            },
            preferred_hero_type: hero_type_from_db(&preferred_hero_type),
            board_visual_mode: board_visual_mode_from_db(&board_visual_mode),
            total_xp,
        })
        .map(Some)
    }

    pub fn current_profile(
        &self,
        session_token: &str,
    ) -> Result<Option<AccountProfile>, IdentityError> {
        self.connection
            .query_row(
                "
                SELECT users.id, users.public_handle, users.email, users.display_name, users.avatar_symbol, users.avatar_color, users.preferred_hero_type, users.board_visual_mode, users.total_xp
                FROM auth_sessions
                JOIN users ON users.id = auth_sessions.user_id
                WHERE auth_sessions.token = ?1
                    AND auth_sessions.expires_at > unixepoch()
                ",
                params![session_token],
                |row| {
                    Ok(AccountProfile {
                        id: row.get(0)?,
                        public_handle: row.get(1)?,
                        email: row.get(2)?,
                        display_name: row.get(3)?,
                        avatar: GeneratedAvatar {
                            symbol: row.get(4)?,
                            color: row.get(5)?,
                        },
                        preferred_hero_type: hero_type_from_db(&row.get::<_, String>(6)?),
                        board_visual_mode: board_visual_mode_from_db(&row.get::<_, String>(7)?),
                        total_xp: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(IdentityError::from)
    }

    pub fn update_profile(
        &mut self,
        user_id: i64,
        public_handle: &str,
        display_name: &str,
        avatar: GeneratedAvatar,
        preferred_hero_type: HeroType,
        board_visual_mode: BoardVisualMode,
    ) -> Result<Option<AccountProfile>, IdentityError> {
        let public_handle = normalize_requested_public_handle(public_handle)?;
        if public_handle_is_taken(self.connection, user_id, &public_handle)? {
            return Err(IdentityError::Validation(
                "Public handle is already taken.".to_string(),
            ));
        }
        self.connection.execute(
            "
            UPDATE users
            SET public_handle = ?2,
                display_name = ?3,
                avatar_symbol = ?4,
                avatar_color = ?5,
                preferred_hero_type = ?6,
                board_visual_mode = ?7
            WHERE id = ?1
            ",
            params![
                user_id,
                public_handle,
                display_name,
                avatar.symbol,
                avatar.color,
                hero_type_to_db(preferred_hero_type),
                board_visual_mode_to_db(board_visual_mode)
            ],
        )?;
        preferences::sync_board_visual_mode_for_user(self.connection, user_id, board_visual_mode)
            .map_err(preferences_error_to_identity_error)?;
        self.load_profile_by_id(user_id)
    }

    pub fn update_preferred_hero(
        &mut self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Option<AccountProfile>, IdentityError> {
        self.connection.execute(
            "UPDATE users SET preferred_hero_type = ?2 WHERE id = ?1",
            params![user_id, hero_type_to_db(hero_type)],
        )?;
        self.load_profile_by_id(user_id)
    }

    pub fn public_profile_by_handle(
        &self,
        public_handle: &str,
    ) -> Result<Option<PublicAccountProfile>, IdentityError> {
        self.connection
            .query_row(
                "
                SELECT id, public_handle, display_name, avatar_symbol, avatar_color
                FROM users
                WHERE public_handle = ?1
                ",
                params![public_handle],
                |row| {
                    Ok(PublicAccountProfile {
                        id: row.get(0)?,
                        public_handle: row.get(1)?,
                        display_name: row.get(2)?,
                        avatar: GeneratedAvatar {
                            symbol: row.get(3)?,
                            color: row.get(4)?,
                        },
                    })
                },
            )
            .optional()
            .map_err(IdentityError::from)
    }

    pub fn logout(&mut self, session_token: &str) -> Result<(), IdentityError> {
        self.connection.execute(
            "DELETE FROM auth_sessions WHERE token = ?1",
            params![session_token],
        )?;
        Ok(())
    }

    fn create_session(
        &mut self,
        profile: AccountProfile,
    ) -> Result<CreatedAuthSession, IdentityError> {
        let token = random_auth_token();
        self.connection.execute(
            "
            INSERT INTO auth_sessions (
                token,
                user_id,
                created_at,
                expires_at
            )
            VALUES (?1, ?2, unixepoch(), unixepoch() + 2592000)
            ",
            params![token, profile.id],
        )?;

        Ok(CreatedAuthSession { token, profile })
    }

    fn load_profile_by_normalized_email(
        &self,
        normalized_email: &str,
    ) -> Result<Option<AccountProfile>, IdentityError> {
        self.connection
            .query_row(
                "
                SELECT id, public_handle, email, display_name, avatar_symbol, avatar_color, preferred_hero_type, board_visual_mode, total_xp
                FROM users
                WHERE email_normalized = ?1
                ",
                params![normalized_email],
                |row| {
                    Ok(AccountProfile {
                        id: row.get(0)?,
                        public_handle: row.get(1)?,
                        email: row.get(2)?,
                        display_name: row.get(3)?,
                        avatar: GeneratedAvatar {
                            symbol: row.get(4)?,
                            color: row.get(5)?,
                        },
                        preferred_hero_type: hero_type_from_db(&row.get::<_, String>(6)?),
                        board_visual_mode: board_visual_mode_from_db(&row.get::<_, String>(7)?),
                        total_xp: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(IdentityError::from)
    }

    fn load_profile_by_id(&self, user_id: i64) -> Result<Option<AccountProfile>, IdentityError> {
        self.connection
            .query_row(
                "
                SELECT id, public_handle, email, display_name, avatar_symbol, avatar_color, preferred_hero_type, board_visual_mode, total_xp
                FROM users
                WHERE id = ?1
                ",
                params![user_id],
                |row| {
                    Ok(AccountProfile {
                        id: row.get(0)?,
                        public_handle: row.get(1)?,
                        email: row.get(2)?,
                        display_name: row.get(3)?,
                        avatar: GeneratedAvatar {
                            symbol: row.get(4)?,
                            color: row.get(5)?,
                        },
                        preferred_hero_type: hero_type_from_db(&row.get::<_, String>(6)?),
                        board_visual_mode: board_visual_mode_from_db(&row.get::<_, String>(7)?),
                        total_xp: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(IdentityError::from)
    }
}

pub fn migrate(connection: &Connection) -> Result<(), IdentityError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL,
            email_normalized TEXT NOT NULL UNIQUE,
            public_handle TEXT UNIQUE,
            password_hash TEXT NOT NULL,
            display_name TEXT NOT NULL DEFAULT '',
            avatar_symbol TEXT NOT NULL DEFAULT 'sparkles',
            avatar_color TEXT NOT NULL DEFAULT 'emerald',
            preferred_hero_type TEXT NOT NULL DEFAULT 'runekeeper',
            board_visual_mode TEXT NOT NULL DEFAULT '3d',
            total_xp INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );
        CREATE TABLE IF NOT EXISTS auth_sessions (
            token TEXT PRIMARY KEY NOT NULL,
            user_id INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            expires_at INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS auth_sessions_user_id_idx
            ON auth_sessions(user_id);
        ",
    )?;
    add_column_if_missing(connection, "users", "public_handle", "TEXT")?;
    add_column_if_missing(
        connection,
        "users",
        "display_name",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        connection,
        "users",
        "avatar_symbol",
        "TEXT NOT NULL DEFAULT 'sparkles'",
    )?;
    add_column_if_missing(
        connection,
        "users",
        "avatar_color",
        "TEXT NOT NULL DEFAULT 'emerald'",
    )?;
    add_column_if_missing(
        connection,
        "users",
        "preferred_hero_type",
        "TEXT NOT NULL DEFAULT 'runekeeper'",
    )?;
    if column_exists(connection, "users", "preferred_wizard_type")? {
        connection.execute(
            "
            UPDATE users
            SET preferred_hero_type = COALESCE(preferred_wizard_type, preferred_hero_type)
            ",
            [],
        )?;
        drop_column_if_exists(connection, "users", "preferred_wizard_type")?;
    }
    add_column_if_missing(
        connection,
        "users",
        "board_visual_mode",
        "TEXT NOT NULL DEFAULT '3d'",
    )?;
    add_column_if_missing(
        connection,
        "users",
        "total_xp",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    connection.execute(
        "
        UPDATE users
        SET display_name = substr(email, 1, instr(email, '@') - 1)
        WHERE display_name = ''
            AND instr(email, '@') > 1
        ",
        [],
    )?;
    backfill_public_handles(connection)?;
    connection.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS users_public_handle_idx ON users(public_handle)",
        [],
    )?;
    preferences::migrate(connection).map_err(preferences_error_to_identity_error)?;
    Ok(())
}

fn preferences_error_to_identity_error(error: preferences::PreferencesError) -> IdentityError {
    match error {
        preferences::PreferencesError::Sqlite(error) => IdentityError::Sqlite(error),
        other => IdentityError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(other))),
    }
}

#[cfg(debug_assertions)]
pub fn seed_experienced_local_account(connection: &Connection) -> Result<i64, IdentityError> {
    let password_hash = hash_password(EXPERIENCED_LOCAL_PASSWORD)?;
    connection.execute(
        "
        INSERT INTO users (
            id,
            email,
            email_normalized,
            password_hash,
            display_name,
            avatar_symbol,
            avatar_color,
            preferred_hero_type,
            total_xp,
            created_at
        )
        VALUES (?1, ?2, ?2, ?3, 'Experienced', 'sparkles', 'emerald', 'runekeeper', ?4, unixepoch())
        ON CONFLICT(email_normalized) DO UPDATE SET
            email = excluded.email,
            password_hash = excluded.password_hash,
            display_name = excluded.display_name,
            avatar_symbol = excluded.avatar_symbol,
            avatar_color = excluded.avatar_color,
            preferred_hero_type = excluded.preferred_hero_type,
            total_xp = excluded.total_xp
        ",
        params![
            EXPERIENCED_LOCAL_USER_ID,
            EXPERIENCED_LOCAL_EMAIL,
            password_hash,
            EXPERIENCED_LOCAL_XP
        ],
    )?;

    let user_id = connection
        .query_row(
            "SELECT id FROM users WHERE email_normalized = ?1",
            params![EXPERIENCED_LOCAL_EMAIL],
            |row| row.get(0),
        )
        .map_err(IdentityError::from)?;
    let public_handle = unique_public_handle(connection, user_id, "Experienced")?;
    connection.execute(
        "UPDATE users SET public_handle = ?2 WHERE id = ?1",
        params![user_id, public_handle],
    )?;

    Ok(user_id)
}

pub fn normalized_email(email: &str) -> Option<(String, String)> {
    let trimmed = email.trim();
    if trimmed.is_empty() || !trimmed.contains('@') {
        return None;
    }
    let normalized = trimmed.to_ascii_lowercase();
    Some((trimmed.to_string(), normalized))
}

fn default_display_name(email: &str) -> String {
    let name = email.split('@').next().unwrap_or("Player").trim();
    if name.is_empty() {
        "Player".to_string()
    } else {
        name.to_string()
    }
}

fn backfill_public_handles(connection: &Connection) -> Result<(), IdentityError> {
    let mut statement = connection.prepare(
        "
        SELECT id, display_name, email
        FROM users
        WHERE public_handle IS NULL OR public_handle = ''
        ORDER BY id ASC
        ",
    )?;
    let users = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for (user_id, display_name, email) in users {
        let seed = if display_name.trim().is_empty() {
            default_display_name(&email)
        } else {
            display_name
        };
        let public_handle = unique_public_handle(connection, user_id, &seed)?;
        connection.execute(
            "UPDATE users SET public_handle = ?2 WHERE id = ?1",
            params![user_id, public_handle],
        )?;
    }

    Ok(())
}

fn unique_public_handle(
    connection: &Connection,
    user_id: i64,
    seed: &str,
) -> Result<String, IdentityError> {
    let base = generated_public_handle(seed, user_id);
    if !public_handle_is_taken(connection, user_id, &base)? {
        return Ok(base);
    }

    for attempt in 0..100 {
        let suffix = if attempt == 0 {
            format!("-{user_id}")
        } else {
            format!("-{user_id}-{attempt}")
        };
        let prefix_len = 24usize.saturating_sub(suffix.len());
        let mut prefix = base.chars().take(prefix_len).collect::<String>();
        prefix = prefix.trim_matches('-').to_string();
        let candidate = format!("{prefix}{suffix}");
        if !public_handle_is_taken(connection, user_id, &candidate)? {
            return Ok(candidate);
        }
    }

    Err(IdentityError::Validation(
        "Could not generate a unique public handle.".to_string(),
    ))
}

fn generated_public_handle(seed: &str, user_id: i64) -> String {
    let mut handle = slugify_public_handle(seed);
    if handle.len() < 3 {
        handle = slugify_public_handle(&format!("{handle}-{user_id}"));
    }
    if handle.len() < 3 {
        handle = format!("user-{user_id}");
    }
    trim_public_handle_length(&handle)
}

fn normalize_requested_public_handle(handle: &str) -> Result<String, IdentityError> {
    let normalized = handle.trim().to_ascii_lowercase();
    if public_handle_is_valid(&normalized) {
        return Ok(normalized);
    }

    Err(IdentityError::Validation(
        "Public handle must be 3 to 24 lowercase letters, numbers, or hyphens, and cannot start or end with a hyphen.".to_string(),
    ))
}

fn public_handle_is_valid(handle: &str) -> bool {
    let bytes = handle.as_bytes();
    if !(3..=24).contains(&bytes.len()) {
        return false;
    }
    if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
        return false;
    }
    bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn public_handle_is_taken(
    connection: &Connection,
    user_id: i64,
    public_handle: &str,
) -> Result<bool, IdentityError> {
    let existing: Option<i64> = connection
        .query_row(
            "SELECT id FROM users WHERE public_handle = ?1 AND id != ?2",
            params![public_handle, user_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(existing.is_some())
}

fn slugify_public_handle(seed: &str) -> String {
    let mut handle = String::new();
    let mut last_was_hyphen = false;
    for byte in seed.bytes() {
        let normalized = byte.to_ascii_lowercase();
        if normalized.is_ascii_lowercase() || normalized.is_ascii_digit() {
            handle.push(normalized as char);
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            handle.push('-');
            last_was_hyphen = true;
        }
    }
    trim_public_handle_length(handle.trim_matches('-'))
}

fn trim_public_handle_length(handle: &str) -> String {
    let mut trimmed = handle.chars().take(24).collect::<String>();
    trimmed = trimmed.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "player".to_string()
    } else {
        trimmed
    }
}

fn generated_avatar_for_email(normalized_email: &str) -> GeneratedAvatar {
    const SYMBOLS: [&str; 6] = ["sparkles", "shield", "sword", "wand", "rune", "flame"];
    const COLORS: [&str; 6] = ["emerald", "indigo", "rose", "amber", "sky", "slate"];
    let sum = normalized_email
        .bytes()
        .fold(0usize, |total, byte| total.wrapping_add(byte as usize));
    GeneratedAvatar {
        symbol: SYMBOLS[sum % SYMBOLS.len()].to_string(),
        color: COLORS[(sum / SYMBOLS.len()) % COLORS.len()].to_string(),
    }
}

fn hero_type_to_db(hero_type: HeroType) -> &'static str {
    match hero_type {
        HeroType::Runekeeper => "runekeeper",
        HeroType::Pyromancer => "pyromancer",
        HeroType::Chronomancer => "chronomancer",
        HeroType::Warden => "warden",
        HeroType::Battlemage => "battlemage",
        HeroType::Barbarian => "barbarian",
        HeroType::Archer => "archer",
        HeroType::Builder => "builder",
    }
}

fn hero_type_from_db(value: &str) -> HeroType {
    match value {
        "pyromancer" => HeroType::Pyromancer,
        "chronomancer" => HeroType::Chronomancer,
        "warden" => HeroType::Warden,
        "battlemage" => HeroType::Battlemage,
        "barbarian" => HeroType::Barbarian,
        "archer" => HeroType::Archer,
        "builder" => HeroType::Builder,
        _ => HeroType::Runekeeper,
    }
}

fn board_visual_mode_to_db(mode: BoardVisualMode) -> &'static str {
    match mode {
        BoardVisualMode::TwoD => "2d",
        BoardVisualMode::ThreeD => "3d",
    }
}

fn board_visual_mode_from_db(value: &str) -> BoardVisualMode {
    match value {
        "2d" => BoardVisualMode::TwoD,
        _ => BoardVisualMode::ThreeD,
    }
}

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), IdentityError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }

    connection.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

fn column_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, IdentityError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn drop_column_if_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<(), IdentityError> {
    if column_exists(connection, table, column)? {
        let _ = connection.execute(&format!("ALTER TABLE {table} DROP COLUMN {column}"), []);
    }
    Ok(())
}

fn hash_password(password: &str) -> Result<String, IdentityError> {
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes())?
        .to_string();
    Ok(password_hash)
}

fn password_matches(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn random_auth_token() -> String {
    random_hex_token(32)
}

fn random_hex_token(byte_count: usize) -> String {
    let mut bytes = vec![0_u8; byte_count];
    UnwrapErr(SysRng).fill_bytes(&mut bytes);
    bytes
        .into_iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod password_hash_tests {
    use super::{hash_password, password_matches};

    /// Produced by `argon2` 0.5 (`Argon2::default()`, fixed salt) before the 0.6 upgrade;
    /// stored account hashes in that format must keep verifying.
    const ARGON2_0_5_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$YXJjYW5pYS1maXh0dXJlIQ$xAW/aH8bVDKE6BlAsR40jYFCU6Ew+mu1Mriu7qyeg24";
    const PASSWORD: &str = "correct horse battery staple";

    #[test]
    fn hashes_stored_by_previous_argon2_release_still_verify() {
        assert!(password_matches(PASSWORD, ARGON2_0_5_HASH));
        assert!(!password_matches("wrong password", ARGON2_0_5_HASH));
    }

    #[test]
    fn new_hashes_use_default_argon2id_parameters_with_fresh_salt() {
        let first = hash_password(PASSWORD).expect("hash password");
        let second = hash_password(PASSWORD).expect("hash password");

        assert!(first.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert_ne!(first, second);
        assert!(password_matches(PASSWORD, &first));
        assert!(!password_matches("wrong password", &first));
    }
}
