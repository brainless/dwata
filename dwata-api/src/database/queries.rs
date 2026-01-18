use rusqlite::Connection;
use shared_types::{AgentMessage, AgentSession, AgentToolCall, SessionListItem};

/// Get all sessions ordered by most recent
pub fn get_all_sessions(conn: &Connection) -> anyhow::Result<Vec<SessionListItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, user_prompt, status, started_at
            FROM agent_sessions
            ORDER BY started_at DESC",
    )?;

    let sessions = stmt.query_map([], |row| {
        Ok(SessionListItem {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            user_prompt: row.get(2)?,
            status: row.get(3)?,
            started_at: row.get(4)?,
        })
    })?;

    let mut result = Vec::new();
    for session in sessions {
        result.push(session?);
    }

    Ok(result)
}

/// Get a single session by ID
pub fn get_session(conn: &Connection, session_id: i64) -> anyhow::Result<Option<AgentSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, provider, model, system_prompt, user_prompt, config, status, started_at, ended_at, result, error
            FROM agent_sessions
            WHERE id = ?1"
    )?;

    let session = stmt.query_row([session_id], |row| {
        let config_str: Option<String> = row.get(6)?;
        let config = config_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(AgentSession {
            id: row.get(0)?,
            agent_name: row.get(1)?,
            provider: row.get(2)?,
            model: row.get(3)?,
            system_prompt: row.get(4)?,
            user_prompt: row.get(5)?,
            config,
            status: row.get(7)?,
            started_at: row.get(8)?,
            ended_at: row.get(9)?,
            result: row.get(10)?,
            error: row.get(11)?,
        })
    });

    match session {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get all messages for a session
pub fn get_session_messages(
    conn: &Connection,
    session_id: i64,
) -> anyhow::Result<Vec<AgentMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, role, content, created_at
            FROM agent_messages
            WHERE session_id = ?1
            ORDER BY created_at ASC",
    )?;

    let messages = stmt.query_map([session_id], |row| {
        Ok(AgentMessage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;

    let mut result = Vec::new();
    for message in messages {
        result.push(message?);
    }

    Ok(result)
}

/// Get all tool calls for a session
pub fn get_session_tool_calls(
    conn: &Connection,
    session_id: i64,
) -> anyhow::Result<Vec<AgentToolCall>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, message_id, tool_call_id, tool_name, request, response, status, created_at, completed_at, execution_time_ms, error_details
            FROM agent_tool_calls
            WHERE session_id = ?1
            ORDER BY created_at ASC"
    )?;

    let calls = stmt.query_map([session_id], |row| {
        let request_str: String = row.get(5)?;
        let request = serde_json::from_str(&request_str).map_err(|_| {
            rusqlite::Error::InvalidColumnType(5, request_str.clone(), rusqlite::types::Type::Text)
        })?;

        let response: Option<serde_json::Value> = row
            .get(6)
            .ok()
            .and_then(|s: String| serde_json::from_str(&s).ok());

        Ok(AgentToolCall {
            id: row.get(0)?,
            session_id: row.get(1)?,
            message_id: row.get(2)?,
            tool_call_id: row.get(3)?,
            tool_name: row.get(4)?,
            request,
            response,
            status: row.get(7)?,
            created_at: row.get(8)?,
            completed_at: row.get(9)?,
            execution_time_ms: row.get(10)?,
            error_details: row.get(11)?,
        })
    })?;

    let mut result = Vec::new();
    for call in calls {
        result.push(call?);
    }

    Ok(result)
}
