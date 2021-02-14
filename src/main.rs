use tungstenite::client::{connect, AutoStream};
use tungstenite::{WebSocket, Message};
use url::Url;

fn main() {
    let server_url = Url::parse("wss://ghost.life").unwrap();
    let mut canvas = RemoteCanvas::new(server_url).unwrap();
    for i in 0..256 {
        canvas.set_pixel(i as u8, i as u8, 0);
        canvas.set_pixel(i as u8, 255-i as u8, 0);
    }
}

struct RemoteCanvas {
    stream: WebSocket<AutoStream>,
}

impl RemoteCanvas {
    fn new(server_url: Url) -> Result<Self, String> {
        let (stream, _) = connect(server_url).map_err(|err| err.to_string())?;
        let canvas = RemoteCanvas {
            stream,
        };
        Ok(canvas)
    }

    fn set_pixel(&mut self, x: u8, y: u8, c: u8) {
        let message = Message::Binary(vec![y, x, c]);
        self.stream.write_message(message).unwrap();
    }
}
