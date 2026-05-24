use greetd_ipc::{codec::SyncCodec, AuthMessageType, ErrorType, Request, Response};
use std::{fmt, os::unix::net::UnixStream};

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
) -> Result<(), GreeterAuthError> {
    let cmd = shell_words::split(session_command)
        .map_err(|err| GreeterAuthError::Command(format!("invalid session_command: {err}")))?;

    if cmd.is_empty() {
        return Err(GreeterAuthError::Command(
            "session_command cannot be empty".to_string(),
        ));
    }

    let mut stream = UnixStream::connect(socket_path)
        .map_err(|err| GreeterAuthError::Connect(format!("unable to connect to greetd: {err}")))?;
    Request::CreateSession {
        username: username.to_string(),
    }
    .write_to(&mut stream)
    .map_err(protocol_error)?;

    loop {
        match Response::read_from(&mut stream).map_err(protocol_error)? {
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => {
                eprintln!(
                    "nodalix-greeter: auth prompt type: {}",
                    prompt_name(&auth_message_type)
                );
                let response = match auth_message_type {
                    AuthMessageType::Secret => Some(password.clone()),
                    AuthMessageType::Visible => Some(String::new()),
                    AuthMessageType::Info | AuthMessageType::Error => Some(String::new()),
                };
                let _ = auth_message;
                Request::PostAuthMessageResponse { response }
                    .write_to(&mut stream)
                    .map_err(protocol_error)?;
            }
            Response::Success => break,
            Response::Error {
                error_type,
                description,
            } => return Err(map_greetd_error(error_type, description)),
        }
    }

    drop(password);

    Request::StartSession {
        cmd,
        env: vec![
            "XDG_SESSION_TYPE=wayland".to_string(),
            "XDG_CURRENT_DESKTOP=Hyprland".to_string(),
        ],
    }
    .write_to(&mut stream)
    .map_err(protocol_error)?;

    match Response::read_from(&mut stream).map_err(protocol_error)? {
        Response::Success => Ok(()),
        Response::Error {
            error_type,
            description,
        } => Err(map_greetd_error(error_type, description)),
        Response::AuthMessage { .. } => Err(GreeterAuthError::Protocol(
            "unexpected auth prompt after start_session".to_string(),
        )),
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
