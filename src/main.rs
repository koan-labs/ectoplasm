use futures::stream::{SplitSink, SplitStream, StreamExt};
use futures::SinkExt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;
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
    remote_canvas.fetch().await;
    for y in 0..=255 {
        for x in 0..=255 {
            let c = remote_canvas.get_pixel(x, y).await;
            remote_canvas.set_pixel(x, y, 255 - c).await;
        }
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
    socket_read: Arc<RwLock<SplitStream<WebSocketStream>>>,
    socket_write: Arc<RwLock<SplitSink<WebSocketStream, Message>>>,
    updater_task: task::JoinHandle<()>,
    fetch_triggers: Arc<RwLock<VecDeque<triggered::Trigger>>>,
}

impl RemoteCanvas {
    async fn new(server_url: Url) -> Result<Self, String> {
        let (socket, _) = connect_async(server_url)
            .await
            .map_err(|err| err.to_string())?;
        let (socket_write, socket_read) = {
            let (write, read) = socket.split();
            (Arc::new(RwLock::new(write)), Arc::new(RwLock::new(read)))
        };
        let local_copy = Arc::new(RwLock::new(Canvas::new()));
        let fetch_triggers = Arc::new(RwLock::new(VecDeque::<triggered::Trigger>::new()));
        let updater_task = {
            let local_copy_clone = local_copy.clone();
            let socket_read_clone = socket_read.clone();
            let fetch_triggers_clone = fetch_triggers.clone();
            task::spawn(async move {
                println!("started update listener");
                loop {
                    println!("waiting for message in update listener loop");
                    let message = socket_read_clone
                        .write()
                        .await
                        .next()
                        .await
                        .unwrap()
                        .unwrap();
                    println!("received message");
                    let mut canvas = local_copy_clone.write().await;
                    let is_key_frame = process_update(&mut canvas, &message);
                    if is_key_frame {
                        if let Some(trigger) = fetch_triggers_clone.write().await.pop_front() {
                            trigger.trigger();
                        }
                    }
                }
            })
        };
        let remote_canvas = Self {
            local_copy,
            socket_read,
            socket_write,
            updater_task,
            fetch_triggers,
        };
        Ok(remote_canvas)
    }

    async fn set_pixel(&mut self, x: u8, y: u8, c: u8) {
        self.local_copy.write().await.set_pixel(x, y, c);
        let message = Message::binary(vec![x, y, c]);
        self.socket_write.write().await.send(message).await.unwrap()
    }

    async fn get_pixel(&self, x: u8, y: u8) -> u8 {
        self.local_copy.read().await.get_pixel(x, y)
    }

    async fn fetch(&mut self) {
        let (trigger, listener) = triggered::trigger();
        self.fetch_triggers.write().await.push_back(trigger);
        let message = Message::text("fetch");
        self.socket_write.write().await.send(message).await.unwrap();
        println!("sent fetch command");
        listener.await;
        println!("done awaiting the matching key-frame");
    }
}

// NOTE(mkovacs): Return value indicates whether the update was a key-frame
fn process_update(canvas: &mut Canvas, message: &Message) -> bool {
    println!("received message: {}", message);
    if let Message::Binary(data) = message {
        if !data.is_empty() {
            match data[0] {
                0 => {
                    if (data.len() - 1) % 3 == 0 {
                        let n = (data.len() - 1) / 3;
                        for i in 0..n {
                            let x = data[3 * i + 1];
                            let y = data[3 * i + 2];
                            let c = data[3 * i + 3];
                            canvas.set_pixel(x, y, c);
                        }
                    } else {
                        println!(
                            "[WARN] expected message length of 3n+1, got {}",
                            message.len()
                        );
                    }
                    false
                }
                1 => {
                    if data.len() == 65537 {
                        for y in 0..=255 {
                            for x in 0..=255 {
                                canvas.set_pixel(x as u8, y as u8, data[1 + x + 256 * y]);
                            }
                        }
                    } else {
                        println!(
                            "[WARN] expected message length of 65537, got {}",
                            message.len()
                        );
                    }
                    true
                }
                2 => {
                    // TODO(mkovacs): Handle palette updates
                    false
                }
                _ => false,
            }
        } else {
            println!("[WARN] unexpected empty message: {}", message);
            false
        }
    } else {
        println!("[WARN] unexpected message: {}", message);
        false
    }
}
