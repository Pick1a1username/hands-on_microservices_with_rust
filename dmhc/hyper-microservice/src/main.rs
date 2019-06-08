use std::fmt;
use std::sync::{Arc, Mutex};

use futures::{future, Future};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;
use slab::Slab;

extern crate futures;
extern crate hyper;


type UserId = u64;

struct UserData;

impl fmt::Display for UserData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("{}")
    }
}

type UserDb = Arc<Mutex<Slab<UserData>>>;

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

const USER_PATH: &str = "/user/";

fn main() {
    let addr = ([127, 0, 0, 1], 8080).into();
    // Bind the address
    let builder = Server::bind(&addr);
    //
    let user_db = Arc::new(Mutex::new(Slab::new()));
    // Handle incoming HTTP requests
    let server = builder.serve(move || {
        let user_db = user_db.clone();
        service_fn(move |req| microservice_handler(req, &user_db))
    });

    // Drop any error
    let server = server.map_err(drop);
    // Start server.
    hyper::rt::run(server);
}

fn microservice_handler(req: Request<Body>, user_db: &UserDb)
    -> impl Future<Item=Response<Body>, Error=Error>
{
    let response = {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => {
                Response::new(INDEX.into())
            },
            (method, path) if path.starts_with(USER_PATH) => {
                // unimplemented!();
                let user_id = path.trim_left_matches(USER_PATH)
                    .parse::<UserId>()
                    .ok()
                    .map(|x| x as usize);
                let mut users = user_db.lock().unwrap();
                match (method, user_id) {
                    (&Method::POST, None) => {
                        let id = users.insert(UserData);
                        Response::new(id.to_string().into())
                    },
                    (&Method::POST, Some(_)) => {
                        response_with_code(StatusCode::BAD_REQUEST)
                    },
                    (&Method::GET, Some(id)) => {
                        if let Some(data) = users.get(id) {
                            Response::new(data.to_string().into())
                        } else {
                            response_with_code(StatusCode::NOT_FOUND)
                        }
                    },
                    (&Method::PUT, Some(id)) => {
                        if let Some(user) = users.get_mut(id) {
                            *user = UserData;
                            response_with_code(StatusCode::OK)
                        } else {
                            response_with_code(StatusCode::NOT_FOUND)
                        }
                    },
                    (&Method::DELETE, Some(id)) => {
                        if users.contains(id) {
                            users.remove(id);
                            response_with_code(StatusCode::OK)
                        } else {
                            response_with_code(StatusCode::NOT_FOUND)
                        }
                    },
                    _ => {
                        response_with_code(StatusCode::METHOD_NOT_ALLOWED)
                    },
                }
            },
            _ => {
                response_with_code(StatusCode::NOT_FOUND)
            },
        }
    };
    future::ok(response)
}

fn response_with_code(status_code: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .body(Body::empty())
        .unwrap()
}
