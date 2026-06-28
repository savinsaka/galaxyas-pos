use rusqlite::params;
use tauri::State;

use crate::auth::{generate_token, verify_password};
use crate::db::models::{now_ms, Session, User};
use crate::db::write_audit;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const SESSION_TTL_MS: i64 = 15 * 60 * 1000;

#[tauri::command]
pub fn login(
    state: State<AppState>,
    username: String,
    password: String,
) -> AppResult<Session> {
    let conn = state.pool.get()?;
    let (user, hash): (User, String) = conn
        .query_row(
            "SELECT id, store_id, username, full_name, role, is_active,
                    created_at, updated_at, password_hash
             FROM users WHERE username = ?1 AND is_active = 1",
            params![username],
            |r| Ok((User::from_row(r), r.get::<_, String>("password_hash"))),
        )
        .map_err(|_| AppError::Validation("Username atau password salah".into()))
        .and_then(|(u, h)| Ok((u?, h?)))?;

    if !verify_password(&password, &hash) {
        return Err(AppError::Validation("Username atau password salah".into()));
    }

    let session = Session {
        user: user.clone(),
        token: generate_token(),
        expires_at: now_ms() + SESSION_TTL_MS,
    };
    *state.session.lock().unwrap() = Some(session.clone());
    write_audit(&conn, &user.store_id, &user.id, "login", None, None, None)?;
    Ok(session)
}

#[tauri::command]
pub fn logout(state: State<AppState>) -> AppResult<()> {
    if let Some(session) = state.session.lock().unwrap().take() {
        let conn = state.pool.get()?;
        write_audit(
            &conn,
            &session.user.store_id,
            &session.user.id,
            "logout",
            None,
            None,
            None,
        )?;
    }
    Ok(())
}

#[tauri::command]
pub fn current_session(state: State<AppState>) -> AppResult<Option<Session>> {
    let guard = state.session.lock().unwrap();
    match guard.clone() {
        Some(s) if s.expires_at > now_ms() => Ok(Some(s)),
        _ => Ok(None),
    }
}

#[tauri::command]
pub fn list_users(state: State<AppState>) -> AppResult<Vec<User>> {
    state.require_role(&["admin", "supervisor"])?;
    let conn = state.pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, store_id, username, full_name, role, is_active, created_at, updated_at
         FROM users ORDER BY username",
    )?;
    let rows = stmt
        .query_map([], |r| Ok(User::from_row(r)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<AppResult<Vec<_>>>()?;
    Ok(rows)
}
