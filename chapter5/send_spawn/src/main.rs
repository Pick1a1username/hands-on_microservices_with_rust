extern crate tokio;

use futures::{future, Stream, stream, Future, Sink, IntoFuture};
use futures::sync::{mpsc};

async fn http_get(addr: &str) -> Result<String, std::io::Error> {
    let mut conn = await!(NetwrokStream::connect(addr))?;
    let _ = await!(conn.write_all(b"GET / HTTP/1.0\r\n\r\n"))?;
    let mut buf = vec![0;1024];
    let len = await!(conn.read(&mut buf))?;
    let res = String::from_utf8_lossy(&buf[..len]).to_string();
    Ok(res)
}

fn to_box<T>(fut :T) -> Box<dyn Future<Item=(), Error=()> + Send>
where
    T: IntoFuture,
    T::Future: Send + 'static,
    T::Item: 'static,
    T::Error: 'static,
{
    let fut = fut.into_future().map(drop).map_err(drop);
    Box::new(fut)
}

fn main() {
    send_spawn();
}
