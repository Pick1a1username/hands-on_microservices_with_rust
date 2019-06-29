#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate base64;
#[macro_use]
extern crate base64_serde;
extern crate queryst;
extern crate serde_cbor;

use std::cmp::{max, min};
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::ops::Range;

use base64::STANDARD;
use clap::{crate_authors, crate_description, crate_name, crate_version, Arg, App};
use dotenv::dotenv;
use futures::{future, Future, Stream};
use hyper::{Body, Method, Response, Request, Server, StatusCode};
// use failure::Error;
use hyper::service::service_fn;
use log::{debug, info, trace, warn};
use rand::Rng;
use rand::distributions::{Bernoulli, Normal, Uniform};
use serde_derive::Deserialize;
use serde_json::Value;
use failure::Error;

mod color;

use color::Color;

// The position of this part is not specified clearly in the book.
base64_serde_type!(Base64Standard, STANDARD);

// This part might be mentioned previous chapter?
static INDEX: &[u8] = b"Random Microservice";

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum RngResponse {
    Value(f64),
    #[serde(with = "Base64Standard")]
    Bytes(Vec<u8>),
    Color(Color),
}

#[derive(Deserialize)]
#[serde(tag = "distribution", content = "parameters", rename_all = "lowercase")]
enum RngRequest {
    Uniform {
        #[serde(flatten)]
        range: Range<i32>,
    },
    Normal {
        mean: f64,
        std_dev: f64,
    },
    Bernoulli {
        p: f64,
    },
    Shuffle {
        #[serde(with = "Base64Standard")]
        data: Vec<u8>,
    },
    Color {
        from: Color,
        to: Color,
    },
}

#[derive(Deserialize)]
struct Config {
    address: SocketAddr,
}

// This part is not mentioned in 'Data formats for interaction with microservices in chapter 4.
fn handle_request(request: RngRequest) -> RngResponse {
    let mut rng = rand::thread_rng();
    match request {
        RngRequest::Uniform { range } => {
            let value = rng.sample(Uniform::from(range)) as f64;
            debug!("(Uniform) Generated value is: {}", value);
            RngResponse::Value(value)
        },
        RngRequest::Normal { mean, std_dev } => {
            let value = rng.sample(Normal::new(mean, std_dev)) as f64;
            debug!("(Normal) Generated value is: {}", value);
            RngResponse::Value(value)
        },
        RngRequest::Bernoulli { p } => {
            let value = rng.sample(Bernoulli::new(p)) as i8 as f64;
            debug!("(Bernoulli) Generated value is: {}", value);
            RngResponse::Value(value)
        },
        RngRequest::Shuffle { mut data } => {
            rng.shuffle(&mut data);
            debug!("(Shuffle) The data is shuffled.");
            RngResponse::Bytes(data)
        },
        RngRequest::Color { from, to } => {
            let red = rng.sample(color_range(from.red, to.red));
            let green = rng.sample(color_range(from.green, to.green));
            let blue = rng.sample(color_range(from.blue, to.blue));
            debug!("(Color) Generated values are: Red -> {}, Green -> {}, Blue -> {}"
                , red
                , green
                , blue
            );
            RngResponse::Color(Color { red, green, blue})
        },
    }
}

fn color_range(from: u8, to: u8) -> Uniform<u8> {
    let (from, to) = (min(from, to), max(from, to));
    Uniform::new_inclusive(from, to)
}

fn microservice_handler(req: Request<Body>)
    -> Box<Future<Item=Response<Body>, Error=hyper::Error> + Send>
{
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") | (&Method::GET, "/random") => {
            Box::new(future::ok(Response::new(INDEX.into())))
        },
        (&Method::POST, "/random") => {
            let format = {
                let uri = req.uri().query().unwrap_or("");
                let query = queryst::parse(uri).unwrap_or(Value::Null);
                query["format"].as_str().unwrap_or("json").to_string()
            };
            let body = req.into_body().concat2()
                .map(move |chunks| {
                    let res = serde_json::from_slice::<RngRequest>(chunks.as_ref())
                        .map(handle_request)
                        .map_err(Error::from)
                        .and_then(move |resp| serialize(&format, &resp));
                    match res {
                        Ok(body) => {
                            Response::new(body.into())
                        },
                        Err(err) => {
                            Response::builder()
                                .status(StatusCode::UNPROCESSABLE_ENTITY)
                                .body(err.to_string().into())
                                .unwrap()
                        },
                    }
                });
            Box::new(body)
        },
        _ => {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Not Found".into())
                .unwrap();
            Box::new(future::ok(resp))
        },
    }
}

fn serialize(format: &str, resp: &RngResponse) -> Result<Vec<u8>, Error> {
    match format {
        "json" => {
            Ok(serde_json::to_vec(resp)?)
        },
        "cbor" => {
            Ok(serde_cbor::to_vec(resp)?)
        },
        _ => {
            // 'format_err!' is the macro of the 'failure' crate.
            Err(format_err!("unsupported format {}", format))
        },
    }
}

fn main() {
    dotenv().ok();
    env_logger::init();

    info!("Rand Microservice - v0.1.0");
    trace!("Starting...");
    
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("address")
             .short("a")
             .long("address")
             .value_name("ADDRESS")
             .help("Sets an address")
             .takes_value(true))
        .arg(Arg::with_name("config")
             .short("c")
             .long("config")
             .value_name("FILE")
             .help("Sets a custom config file")
             .takes_value(true))
        .get_matches();

    let config = File::open("microservice.toml")
        .and_then(|mut file| {
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            Ok(buffer)
        })
        .and_then(|buffer| {
            toml::from_str::<Config>(&buffer)
                .map_err(|err|
                    io::Error::new(io::ErrorKind::Other, err))
        })
        .map_err(|err| {
            warn!("Can't read config file: {}", err);
        })
        .ok();

    let addr = matches.value_of("address")
        .map(|s| s.to_owned())
        .or(env::var("ADDRESS").ok())
        .and_then(|addr| addr.parse().ok())
        .or(config.map(|config| config.address))
        .or_else(|| Some(([127, 0, 0, 1], 8080).into()))
        .unwrap();



    debug!("Trying to bind server to address: {}", addr);
    let builder = Server::bind(&addr);
    trace!("Creating service handler...");
    let server = builder.serve(|| {
        service_fn(microservice_handler)
        // service_fn_ok(|req| {
        //     trace!("Incoming request is: {:?}", req);
        //     let random_byte = rand::random::<u8>();
        //     debug!("Generated value is: {}", random_byte);
        //     Response::new(Body::from(random_byte.to_string()))
        // })
    });

    info!("Used address: {}", server.local_addr());
    let server = server.map_err(drop);
    debug!("Run!");
    hyper::rt::run(server);
}
