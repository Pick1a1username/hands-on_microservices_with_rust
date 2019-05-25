use hyper::{Body, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn_ok;


fn main() {
    let addr = ([127, 0, 0, 1], 8080).into();
    // Bind the address
    let builder = Server::bind(&addr);
    // Handle incoming HTTP requests
    let server = builder.serve(|| {
        service_fn_ok(|_| {
            Response::new(Body::from("Almost microservice..."))
            })
        });

    // Drop any error
    let server = server.map_err(drop);
    // Start server.
    hyper::rt::run(server);
}
