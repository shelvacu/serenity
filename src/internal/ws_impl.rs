use flate2::read::ZlibDecoder;
use crate::model::event::WsEvent;
use crate::gateway::WsClient;
use crate::internal::prelude::*;
use serde_json;
use tungstenite::{
    util::NonBlockingResult,
    Message,
};
use log::warn;

#[cfg(not(feature = "native_tls_backend"))]
use std::{
    error::Error as StdError,
    fmt::{
        Display,
        Formatter,
        Result as FmtResult,
    },
    io::Error as IoError,
    net::TcpStream,
    sync::Arc,
};
#[cfg(not(feature = "native_tls_backend"))]
use url::Url;

pub trait ReceiverExt {
    fn recv_json(&mut self)     -> Result<Option<(WsEvent, Result<Value>)>>;
    fn try_recv_json(&mut self) -> Result<Option<(WsEvent, Result<Value>)>>;
}

pub trait SenderExt {
    fn send_json(&mut self, value: &Value) -> Result<()>;
}

impl ReceiverExt for WsClient {
    fn recv_json(&mut self) -> Result<Option<(WsEvent, Result<Value>)>> {
        Ok(convert_ws_message(Some(self.read_message()?)))
    }

    fn try_recv_json(&mut self) -> Result<Option<(WsEvent, Result<Value>)>> {
        Ok(convert_ws_message(self.read_message().no_block()?))
    }
}

impl SenderExt for WsClient {
    fn send_json(&mut self, value: &Value) -> Result<()> {
        serde_json::to_string(value)
            .map(Message::Text)
            .map_err(Error::from)
            .and_then(|m| self.write_message(m).map_err(Error::from))
    }
}

#[inline]
fn convert_ws_message(message: Option<Message>) -> Option<(WsEvent, Result<Value>)>{
    match message {
        None => None,
        Some(msg) => {
            let raw_event;
            #[cfg(feature = "raw-ws-event")]
            {
                let happened_at_instant = std::time::Instant::now();
                let happened_at_chrono = ::chrono::Utc::now();
                raw_event = WsEvent {
                    happened_at_chrono,
                    happened_at_instant,
                    data: msg.clone(),
                }
            }
            #[cfg(not(feature = "raw-ws-event"))]
            {
                raw_event = WsEvent;
            }

            match convert_ws_message_inner(msg).transpose() {
                None => None,
                Some(res) => Some((raw_event, res)),
            }
        }
    }
}
            
#[inline]
fn convert_ws_message_inner(message: Message) -> Result<Option<Value>> {
    Ok(match message {
        Message::Binary(bytes) => {
            serde_json::from_reader(ZlibDecoder::new(&bytes[..]))
                .map(Some)
                .map_err(|why| {
                    warn!("Err deserializing bytes: {:?}; bytes: {:?}", why, bytes);

                    why
                })?
        },
        Message::Text(payload) => {
            serde_json::from_str(&payload).map(Some).map_err(|why| {
                warn!(
                    "Err deserializing text: {:?}; text: {}",
                    why,
                    payload,
                );

                why
            })?
        },
        // Ping/Pong message behaviour is internally handled by tungstenite.
        _ => None,
    })
}

/// An error that occured while connecting over rustls
#[derive(Debug)]
#[cfg(not(feature = "native_tls_backend"))]
pub enum RustlsError {
    /// WebPKI X.509 Certificate Validation Error.
    WebPKI,
    /// An error with the handshake in tungstenite
    HandshakeError,
    /// Standard IO error happening while creating the tcp stream
    Io(IoError),
    #[doc(hidden)]
    #[cfg(not(feature = "allow_exhaustive_enum"))]
    __Nonexhaustive,
}

#[cfg(not(feature = "native_tls_backend"))]
impl From<IoError> for RustlsError {
    fn from(e: IoError) -> Self {
        RustlsError::Io(e)
    }
}

#[cfg(not(feature = "native_tls_backend"))]
impl Display for RustlsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult { f.write_str(self.description()) }
}

#[cfg(not(feature = "native_tls_backend"))]
impl StdError for RustlsError {
    fn description(&self) -> &str {
        use self::RustlsError::*;

        match *self {
            WebPKI => "Failed to validate X.509 certificate",
            HandshakeError => "TLS handshake failed when making the websocket connection",
            Io(ref inner) => inner.description(),
            #[cfg(not(feature = "allow_exhaustive_enum"))]
            __Nonexhaustive => unreachable!(),
        }
    }
}

// Create a tungstenite client with a rustls stream.
#[cfg(not(feature = "native_tls_backend"))]
pub(crate) fn create_rustls_client(url: Url) -> Result<WsClient> {
    let mut config = rustls::ClientConfig::new();
    config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

    let base_host = if let Some(h) = url.host_str() {
        let (dot, _) = h.rmatch_indices('.').nth(1).unwrap_or((0, ""));
        // We do not want the leading '.', but if there is no leading '.' we do
        // not want to remove the leading character.
        let split_at_index = if dot == 0 { 0 } else { dot + 1 };
        let (_, base) = h.split_at(split_at_index);
        base.to_owned()
    } else { "discord.gg".to_owned() };

    let dns_name = webpki::DNSNameRef::try_from_ascii_str(&base_host)
        .map_err(|_| RustlsError::WebPKI)?;

    let session = rustls::ClientSession::new(&Arc::new(config), dns_name);
    let socket = TcpStream::connect(&url)?;
    let tls = rustls::StreamOwned::new(session, socket);

    let client = tungstenite::client(url, tls)
        .map_err(|_| RustlsError::HandshakeError)?;

    Ok(client.0)
}
