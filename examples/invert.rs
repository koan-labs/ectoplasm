use ectoplasm::tokio;
use ectoplasm::url::Url;
use ectoplasm::RemoteCanvas;

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
