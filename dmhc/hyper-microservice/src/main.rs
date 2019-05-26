use futures::{future, Future};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;

extern crate futures;
extern crate hyper;

const INDEX: &'static str = r#"
<!doctype html>
<html>
    <head>
        <title>Rust Microservice</title>
    </head>
    <body>
        <h3>Rust Microservice</h3>
    </body>
</html>
"#;

fn main() {
    let addr = ([127, 0, 0, 1], 8080).into();
    // Bind the address
    let builder = Server::bind(&addr);
    // Handle incoming HTTP requests
    let server = builder.serve(|| service_fn(microservice_handler));

    // Drop any error
    let server = server.map_err(drop);
    // Start server.
    hyper::rt::run(server);
}

fn microservice_handler(req: Request<Body>)
    -> impl Future<Item=Response<Body>, Error=Error>
{
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            future::ok(Response::new(INDEX.into()))
        },
        _ => {
            let response = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap();
            future::ok(response)
        },
    }
}
