extern crate tokio;

use futures::{future, Stream, Future, Sink, IntoFuture};
use futures::sync::{mpsc};


fn multiple() {
    let (tx_sink, rx_stream) = mpsc::channel::<u8>(8);
    let receiver = rx_stream.fold(0, |acc, value| {
        future::ok(acc + value)
    }).map(|x| {
        println!("Calculated: {}", x);
    });

    let send_1 = tx_sink.clone().send(1);
    let send_2 = tx_sink.clone().send(2);
    let send_3 = tx_sink.clone().send(3);

    let execute_all = future::join_all(vec![
        to_box(receiver),
        to_box(send_1),
        to_box(send_2),
        to_box(send_3),
    ]).map(drop);
    drop(tx_sink);
    tokio::run(execute_all);
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
    multiple();
}
