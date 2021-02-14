use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

type WebSocketStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::stream::Stream<
        tokio::net::TcpStream,
        tokio_native_tls::TlsStream<tokio::net::TcpStream>,
    >,
>;

#[tokio::main]
async fn main() {
    let server_url = Url::parse("wss://ghost.life").unwrap();
    let mut remote_canvas = RemoteCanvas::new(server_url).await.unwrap();
    for i in 0..=255 {
        remote_canvas.set_pixel(i, i, 128).await;
        remote_canvas.set_pixel(i, 255 - i, 128).await;
    }
}

struct Canvas {
    buffer: [[u8; 256]; 256],
}

impl Canvas {
    fn new() -> Self {
        Self {
            buffer: [[0; 256]; 256],
        }
    }

    fn get_pixel(&self, x: u8, y: u8) -> u8 {
        self.buffer[y as usize][x as usize]
    }

    fn set_pixel(&mut self, x: u8, y: u8, c: u8) {
        self.buffer[y as usize][x as usize] = c;
    }
}

struct RemoteCanvas {
    local_copy: Arc<RwLock<Canvas>>,
    socket_read: SplitStream<WebSocketStream>,
    socket_write: SplitSink<WebSocketStream, Message>,
}

impl RemoteCanvas {
    async fn new(server_url: Url) -> Result<Self, String> {
        let (socket, _) = connect_async(server_url)
            .await
            .map_err(|err| err.to_string())?;
        let (write, read) = socket.split();
        let canvas = Arc::new(RwLock::new(Canvas::new()));
        let canvas = Self {
            local_copy: canvas,
            socket_read: read,
            socket_write: write,
        };
        Ok(canvas)
    }

    async fn set_pixel(&mut self, x: u8, y: u8, c: u8) {
        let message = Message::Binary(vec![x, y, c]);
        self.socket_write.send(message).await.unwrap();
        self.local_copy.write().await.set_pixel(x, y, c);
    }

    async fn get_pixel(&self, x: u8, y: u8) {
        self.local_copy.read().await.get_pixel(x, y);
    }
}
