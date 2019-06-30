extern crate tokio;

use std::io;

use futures::{future, Stream, Future, Sink, IntoFuture};
use futures::sync::{mpsc};
use tokio::net::{UdpSocket, UdpFramed};
use tokio::codec::LinesCodec;
use failure::Error;

fn alt_udp_echo() -> Result<(), Error> {
    let from = "0.0.0.0:12345".parse()?;
    let socket = UdpSocket::bind(&from)?;
    let framed = UdpFramed::new(socket, LinesCodec::new());
    let (sink, stream) = framed.split();

    let (tx, rx) = mpsc::channel(16);
    let rx = rx.map_err(|_| other("can't take a message"))
        .fold(sink, |sink, frame| {
            sink.send(frame)
        });

    let process = stream.and_then(move |args| {
        tx.clone()
            .send(args)
            .map(drop)
            .map_err(other)
    }).collect();

    let execute_all = future::join_all(vec![
        to_box(rx),
        to_box(process),
    ]).map(drop);
    Ok(tokio::run(execute_all))
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

// For dealing with Future's error and Stream's error.
fn other<E>(err: E) -> io::Error
where
    E: Into<Box<std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::Other, err)
}

fn main() {
    alt_udp_echo().unwrap();
}
