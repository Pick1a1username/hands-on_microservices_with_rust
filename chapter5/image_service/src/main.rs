use std::io::{Error, ErrorKind};
use std::path::Path;
use std::fs;

use futures::{future, Future, Stream};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::service_fn;
use hyper_staticfile::FileChunkStream;
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use regex::Regex;
use tokio::fs::File;


static INDEX: &[u8] = b"Image Service";

lazy_static! {
    static ref DOWNLOAD_FILE: Regex = Regex::new("^/download/(?P<filename>\\w{20})?$").unwrap();
}

fn microservice_handler(req: Request<Body>, files: &Path)
    -> Box<Future<Item=Response<Body>, Error=std::io::Error> + Send>
{
    match (req.method(), req.uri().path().to_owned().as_ref()) {
        (&Method::GET, "/") => {
            Box::new(future::ok(Response::new(INDEX.into())))
        },
        (&Method::POST, "/upload") => {
            // Generate a randome name for the incoming file.
            let name: String = thread_rng().sample_iter(&Alphanumeric).take(20).collect();
            let mut filepath = files.to_path_buf();
            filepath.push(&name);

            // This is tokio::fs::File, not std::fs::File.
            let create_file = File::create(filepath);
            let write = create_file.and_then(|file| {
                req.into_body()
                    .map_err(other) // other is a function.
                    .fold(file, |file, chunk| {
                        tokio::io::write_all(file, chunk)
                            .map(|(file, _)| file)
                        })
            });
            let body = write.map(|_| {
                Response::new(name.into())
            });
            Box::new(body)
        },
        (&Method::GET, path) if path.starts_with("/download") => {
            if let Some(cap) = DOWNLOAD_FILE.captures(path) {
                let filename = cap.name("filename").unwrap().as_str();
                let mut filepath = files.to_path_buf();
                filepath.push(filename);

                let open_file = File::open(filepath);
                let body = open_file.map(|file| {
                    let chunks = FileChunkStream::new(file);
                    Response::new(Body::wrap_stream(chunks))
                });
                Box::new(body)
            } else {
                response_with_code(StatusCode::NOT_FOUND)
            }
        },
        _ => {
            response_with_code(StatusCode::NOT_FOUND)
        },
    }
}

fn response_with_code(status_code: StatusCode)
    -> Box<Future<Item=Response<Body>, Error=Error> + Send>
{
    let resp = Response::builder()
        .status(status_code)
        .body(Body::empty())
        .unwrap();
    Box::new(future::ok(resp))
}

fn other<E>(err: E) -> Error
where
    E: Into<Box<std::error::Error + Send + Sync>>,
{
    Error::new(ErrorKind::Other, err)
}

fn main() {
    // Set a directory that will keep all incoming files.
    let files = Path::new("./files");
    // Create the directory;
    fs::create_dir(files).ok();

    let addr = ([127, 0, 0, 1], 8080).into();
    let builder = Server::bind(&addr);
    let server = builder.serve(move || {
        service_fn(move |req| microservice_handler(req, &files))
    });
    let server = server.map_err(drop);
    hyper::rt::run(server);
}


