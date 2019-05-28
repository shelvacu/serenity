use flate2::read::ZlibDecoder;
use gateway::GatewayError;
use model::event::RawEvent;
use internal::prelude::*;
use serde_json;
use websocket::{
    message::OwnedMessage,
    sync::stream::{TcpStream, TlsStream},
    sync::Client as WsClient
};

pub trait ReceiverExt {
    fn recv_json(&mut self) -> Result<(RawEvent, Result<Option<Value>>)>;
}

pub trait SenderExt {
    fn send_json(&mut self, value: &Value) -> Result<()>;
}

impl ReceiverExt for WsClient<TlsStream<TcpStream>> {
    fn recv_json(&mut self) -> Result<(RawEvent, Result<Option<Value>>)> {
        let owned_msg = self.recv_message()?;
        #[cfg(feature = "raw-ws-event")]
        let happened_at_instant = std::time::Instant::now();
        #[cfg(feature = "raw-ws-event")]
        let happened_at_chrono = ::chrono::Local::now();
        #[cfg(feature = "raw-ws-event")]
        let raw_event = RawEvent {
            happened_at_chrono,
            happened_at_instant,
            data: owned_msg.clone(),
        };
        #[cfg(not(feature = "raw-ws-event"))]
        let raw_event = RawEvent;
        
        let res_2:Result<_> = match owned_msg {
            OwnedMessage::Binary(bytes) => {
                serde_json::from_reader(ZlibDecoder::new(&bytes[..]))
                    .map(Some)
                    .map_err(|why| {
                        warn!("Err deserializing bytes: {:?}; bytes: {:?}", why, bytes);

                        Error::from(why)
                    })
            },
            OwnedMessage::Close(data) => return Err(Error::Gateway(GatewayError::Closed(data))),
            OwnedMessage::Text(payload) => {
                serde_json::from_str(&payload).map(Some).map_err(|why| {
                    warn!(
                        "Err deserializing text: {:?}; text: {}",
                        why,
                        payload,
                    );

                    Error::from(why)
                })
            },
            OwnedMessage::Ping(x) => {
                match self.send_message(&OwnedMessage::Pong(x)) {
                    Ok(_) => Ok(None),
                    Err(v) => Err(Error::from(v)),
                }
            },
            OwnedMessage::Pong(_) => Ok(None),
        };
        return Ok((raw_event, res_2));
    }
}

impl SenderExt for WsClient<TlsStream<TcpStream>> {
    fn send_json(&mut self, value: &Value) -> Result<()> {
        serde_json::to_string(value)
            .map(OwnedMessage::Text)
            .map_err(Error::from)
            .and_then(|m| self.send_message(&m).map_err(Error::from))
    }
}
