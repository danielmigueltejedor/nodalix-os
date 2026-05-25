use crate::logging;
use greetd_ipc::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};
use std::{fmt, os::unix::net::UnixStream};

#[derive(Debug)]
pub enum AuthOutcome {
    Success,
    InvalidPassword,
    SessionStartFailed(String),
    GreetdProtocolError(String),
    ConnectionError(String),
    CommandError(String),
}

impl AuthOutcome {
    pub fn ui_message(&self) -> &'static str {
        match self {
            Self::InvalidPassword => "Contraseña incorrecta",
            Self::SessionStartFailed(_) => "No se pudo iniciar la sesión",
            Self::GreetdProtocolError(_) | Self::ConnectionError(_) | Self::CommandError(_) => {
                "No se pudo iniciar sesión"
            }
            Self::Success => "",
        }
    }
}

#[derive(Debug)]
pub enum GreeterAuthError {
    Connect(String),
    Protocol(String),
    Authentication(String),
    Session(String),
    Command(String),
}

impl fmt::Display for GreeterAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(msg)
            | Self::Protocol(msg)
            | Self::Authentication(msg)
            | Self::Session(msg)
            | Self::Command(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn authenticate_and_start(
    socket_path: &str,
    username: &str,
    password: String,
    session_command: &str,
) -> AuthOutcome {
    logging::log_event(format!("auth started for user: {username}"));
    let cmd = shell_words::split(session_command)
        .map_err(|err| GreeterAuthError::Command(format!("invalid session_command: {err}")));
    let cmd = match cmd {
        Ok(cmd) => cmd,
        Err(err) => return outcome_from_error(err),
    };

    if cmd.is_empty() {
        return outcome_from_error(GreeterAuthError::Command(
            "session_command cannot be empty".to_string(),
        ));
    }

    let mut stream = match UnixStream::connect(socket_path) {
        Ok(stream) => stream,
        Err(err) => {
            return outcome_from_error(GreeterAuthError::Connect(format!(
                "unable to connect to greetd: {err}"
            )))
        }
    };
    if let Err(err) = (Request::CreateSession {
        username: username.to_string(),
    })
    .write_to(&mut stream)
    .map_err(protocol_error)
    {
        return outcome_from_error(err);
    }

    loop {
        match Response::read_from(&mut stream).map_err(protocol_error) {
            Err(err) => return outcome_from_error(err),
            Ok(response) => match response {
                Response::AuthMessage {
                    auth_message_type,
                    auth_message,
                } => {
                    logging::log_event(format!(
                        "auth message type received: {}",
                        prompt_name(&auth_message_type)
                    ));
                    let response = match auth_message_type {
                        AuthMessageType::Secret => Some(password.clone()),
                        AuthMessageType::Visible => Some(String::new()),
                        AuthMessageType::Info | AuthMessageType::Error => Some(String::new()),
                    };
                    let _ = auth_message;
                    if let Err(err) = (Request::PostAuthMessageResponse { response })
                        .write_to(&mut stream)
                        .map_err(protocol_error)
                    {
                        return outcome_from_error(err);
                    }
                }
                Response::Success => {
                    logging::log_event("auth success");
                    break;
                }
                Response::Error {
                    error_type,
                    description,
                } => return outcome_from_error(map_greetd_error(error_type, description)),
            },
        }
    }

    drop(password);

    logging::log_event(format!("session start requested: {session_command}"));
    if let Err(err) = (Request::StartSession {
        cmd,
        env: vec![
            "XDG_SESSION_TYPE=wayland".to_string(),
            "XDG_CURRENT_DESKTOP=Hyprland".to_string(),
        ],
    })
    .write_to(&mut stream)
    .map_err(protocol_error)
    {
        return outcome_from_error(err);
    }

    match Response::read_from(&mut stream).map_err(protocol_error) {
        Ok(Response::Success) => {
            logging::log_event("session start accepted");
            AuthOutcome::Success
        }
        Ok(Response::Error {
            error_type,
            description,
        }) => outcome_from_error(map_greetd_error(error_type, description)),
        Ok(Response::AuthMessage { .. }) => outcome_from_error(GreeterAuthError::Protocol(
            "unexpected auth prompt after start_session".to_string(),
        )),
        Err(err) => {
            logging::log_event(format!(
                "socket closed or IPC ended after session start request; treating as success: {err}"
            ));
            AuthOutcome::Success
        }
    }
}

fn outcome_from_error(error: GreeterAuthError) -> AuthOutcome {
    match error {
        GreeterAuthError::Authentication(description) => {
            logging::log_event(format!("auth failure: {description}"));
            AuthOutcome::InvalidPassword
        }
        GreeterAuthError::Session(description) => {
            logging::log_event(format!("session start failed: {description}"));
            AuthOutcome::SessionStartFailed(description)
        }
        GreeterAuthError::Protocol(description) => {
            logging::log_event(format!("greetd protocol error: {description}"));
            AuthOutcome::GreetdProtocolError(description)
        }
        GreeterAuthError::Connect(description) => {
            logging::log_event(format!("connection error: {description}"));
            AuthOutcome::ConnectionError(description)
        }
        GreeterAuthError::Command(description) => {
            logging::log_event(format!("command error: {description}"));
            AuthOutcome::CommandError(description)
        }
    }
}

impl fmt::Display for AuthOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::InvalidPassword => write!(f, "invalid password"),
            Self::SessionStartFailed(msg)
            | Self::GreetdProtocolError(msg)
            | Self::ConnectionError(msg)
            | Self::CommandError(msg) => write!(f, "{msg}"),
        }
    }
}

fn protocol_error(err: greetd_ipc::codec::Error) -> GreeterAuthError {
    GreeterAuthError::Protocol(format!("greetd IPC error: {err}"))
}

fn map_greetd_error(error_type: ErrorType, description: String) -> GreeterAuthError {
    match error_type {
        ErrorType::AuthError => GreeterAuthError::Authentication(description),
        ErrorType::Error => GreeterAuthError::Session(description),
    }
}

fn prompt_name(message_type: &AuthMessageType) -> &'static str {
    match message_type {
        AuthMessageType::Visible => "visible",
        AuthMessageType::Secret => "secret",
        AuthMessageType::Info => "info",
        AuthMessageType::Error => "error",
    }
}
