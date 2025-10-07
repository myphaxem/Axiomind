use crate::session::{
    GameConfig, GameStateResponse, OpponentType, SessionError, SessionId, SessionManager,
};
use axm_engine::player::PlayerAction;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::http::{self, StatusCode};
use warp::reply::{self, Response};
use warp::Reply;

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub seed: Option<u64>,
    pub level: Option<u8>,
    pub opponent_type: Option<OpponentType>,
}

impl CreateSessionRequest {
    fn into_config(self) -> GameConfig {
        let mut config = GameConfig::default();
        if let Some(seed) = self.seed {
            config.seed = Some(seed);
        }
        if let Some(level) = self.level {
            config.level = level;
        }
        if let Some(opponent_type) = self.opponent_type {
            config.opponent_type = opponent_type;
        }
        config
    }
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: SessionId,
    pub config: GameConfig,
    pub state: GameStateResponse,
}

#[derive(Debug, Deserialize)]
pub struct PlayerActionRequest {
    pub action: PlayerAction,
}

pub async fn create_session(
    sessions: Arc<SessionManager>,
    request: CreateSessionRequest,
) -> Response {
    let config = request.into_config();

    match sessions.create_session(config.clone()) {
        Ok(session_id) => match sessions.state(&session_id) {
            Ok(state) => success_response(
                StatusCode::CREATED,
                SessionResponse {
                    session_id,
                    config,
                    state,
                },
            ),
            Err(err) => {
                let _ = sessions.delete_session(&session_id);
                session_error(err)
            }
        },
        Err(err) => session_error(err),
    }
}

pub async fn get_session(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match assemble_session_response(&sessions, &session_id) {
        Ok(response) => success_response(StatusCode::OK, response),
        Err(err) => session_error(err),
    }
}

pub async fn get_session_state(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match sessions.state(&session_id) {
        Ok(state) => success_response(StatusCode::OK, state),
        Err(err) => session_error(err),
    }
}

pub async fn submit_action(
    sessions: Arc<SessionManager>,
    session_id: SessionId,
    request: PlayerActionRequest,
) -> Response {
    match sessions.process_action(&session_id, request.action) {
        Ok(event) => success_response(StatusCode::ACCEPTED, event),
        Err(err) => session_error(err),
    }
}

pub async fn delete_session(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match sessions.delete_session(&session_id) {
        Ok(()) => empty_response(StatusCode::NO_CONTENT),
        Err(err) => session_error(err),
    }
}

fn assemble_session_response(
    sessions: &SessionManager,
    session_id: &SessionId,
) -> Result<SessionResponse, SessionError> {
    let config = sessions.config(session_id)?;
    let state = sessions.state(session_id)?;
    Ok(SessionResponse {
        session_id: session_id.clone(),
        config,
        state,
    })
}

fn success_response<T>(status: StatusCode, body: T) -> Response
where
    T: Serialize,
{
    reply::with_status(reply::json(&body), status).into_response()
}

fn empty_response(status: StatusCode) -> Response {
    http::Response::builder()
        .status(status)
        .body(warp::hyper::Body::empty())
        .expect("build empty response")
}

fn session_error(err: SessionError) -> Response {
    let (status, error_code) = match err {
        SessionError::NotFound(_) => (StatusCode::NOT_FOUND, "session_not_found"),
        SessionError::Expired(_) => (StatusCode::NOT_FOUND, "session_expired"),
        SessionError::InvalidAction(_) => (StatusCode::BAD_REQUEST, "invalid_action"),
        SessionError::EngineError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "engine_error"),
        SessionError::StoragePoisoned => {
            (StatusCode::INTERNAL_SERVER_ERROR, "session_storage_error")
        }
    };
    error_response(status, error_code, err.to_string())
}

fn error_response(status: StatusCode, error: &'static str, message: String) -> Response {
    #[derive(Serialize)]
    struct ErrorBody<'a> {
        error: &'a str,
        message: String,
    }

    let body = ErrorBody { error, message };
    reply::with_status(reply::json(&body), status).into_response()
}
