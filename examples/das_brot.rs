use ectoplasm::tokio;
use ectoplasm::url::Url;
use ectoplasm::RemoteCanvas;
use num_complex::Complex32;
use rand::Rng;

#[tokio::main]
async fn main() {
    let server_url = Url::parse("wss://ghost.life").unwrap();
    let mut remote_canvas = RemoteCanvas::new(server_url).await.unwrap();
    remote_canvas.fetch().await;
    let mut rng = rand::thread_rng();
    let dist = num_complex::ComplexDistribution::new(
        rand::distributions::Uniform::new(-1.5, 0.5),
        rand::distributions::Uniform::new(-1.0, 1.0),
    );
    loop {
        let c = rng.sample(dist);
        let mut z = Complex32::new(0.0, 0.0);
        for _ in 0..200 {
            let w = z * z + c;
            if w.re < -1.5 || w.re > 0.5 || w.im < -1.0 || w.im > 1.0 {
                break;
            }
            z = w;
        }
        let (cx, cy) = complex_to_coords(c);
        let (zx, zy) = complex_to_coords(z);
        let index = remote_canvas.get_pixel(zx, zy).await;
        remote_canvas.set_pixel(cx, cy, index).await;
    }
}

fn complex_to_coords(c: Complex32) -> (u8, u8) {
    let x = (128.0 * (c.re + 1.5)) as u8;
    let y = (128.0 * (c.im + 1.0)) as u8;
    (x, y)
}
