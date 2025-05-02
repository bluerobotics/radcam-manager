use axum::{
    Router,
    extract::{
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use tracing::*;

use crate::logger;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new().route("/", get(log_websocket_handler))
}

async fn log_websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(log_websocket_connection)
}

#[instrument(level = "debug", skip_all)]
async fn log_websocket_connection(socket: WebSocket) {
    let (mut websocket_sender, mut _websocket_receiver) = socket.split();
    let (mut receiver, history) = logger::HISTORY.get().unwrap().lock().unwrap().subscribe();

    for message in history {
        if websocket_sender
            .send(Message::Text(message.into()))
            .await
            .is_err()
        {
            return;
        }
    }

    while let Ok(message) = receiver.recv().await {
        if websocket_sender
            .send(Message::Text(message.into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
