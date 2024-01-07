use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use dotenv::dotenv;
use rand::prelude::*;
use ws2818_rgb_led_spi_driver::adapter_gen::WS28xxAdapter;
use ws2818_rgb_led_spi_driver::adapter_spi::WS28xxSpiAdapter;

use solar_status::SolarStatusDisplay;

use crate::error::SolarMonitorError;
use crate::rgbdigit::SevenSegmentDisplayString;
use crate::tesla_powerwall::PowerwallApi;

use std::convert::Infallible;
use std::net::SocketAddr;
use http_body_util::combinators::BoxBody;

use hyper_util::rt::{TokioIo, TokioTimer};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use tokio::net::TcpListener;

#[cfg(feature = "i2c_display")]
mod i2c_display;

mod solar_status;

#[cfg(not(feature = "i2c_display"))]
mod console_display;

mod error;
mod rgbdigit;
mod rgbdigit_display;
mod tesla_powerwall;

async fn main_loop(
    mut powerwall: PowerwallApi,
    display: Box<&mut dyn SolarStatusDisplay>,
    shutdown_signal: Arc<AtomicBool>,
) -> Result<(), SolarMonitorError> {
    powerwall.wait_for_connection().await?;

    loop {
        let status = powerwall.get_stats().await?;

        display.show_status(status)?;

        thread::sleep(time::Duration::from_millis(1_000));

        if shutdown_signal.load(Ordering::Relaxed) {
            break;
        }
    }

    Ok(())
}

// An async function that consumes a request, does nothing with it and returns a
// response.
async fn hello(_: Request<impl hyper::body::Body>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello World!"))))
}

async fn echo(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => Ok(Response::new(full(
            "Try PUT to /start or /stop",
        ))),
        (&Method::PUT, "/start") => {

            println!("Starting solar monitor");
            Ok(Response::new(full(
                "Starting solar monitor...",
            )))
        },
        (&Method::PUT, "/stop") => {
            println!("Stopping solar monitor");
            Ok(Response::new(full(
                "Stopping solar monitor...",
            )))
        },

        // Return 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

// We create some utility functions to make Empty and Full bodies
// fit our broadened Response body type.
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    // This address is localhost
    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();

    // Bind to the port and listen for incoming TCP connections
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        // When an incoming TCP connection is received grab a TCP stream for
        // client<->server communication.
        //
        // Note, this is a .await point, this loop will loop forever but is not a busy loop. The
        // .await point allows the Tokio runtime to pull the task off of the thread until the task
        // has work to do. In this case, a connection arrives on the port we are listening on and
        // the task is woken up, at which point the task is then put back on a thread, and is
        // driven forward by the runtime, eventually yielding a TCP stream.
        let (tcp, _) = listener.accept().await?;
        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(tcp);

        // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
        // current task without waiting for the processing of the HTTP1 connection we just received
        // to finish
        tokio::task::spawn(async move {
            // Handle the connection from the client using HTTP1 and pass any
            // HTTP requests received on that connection to the `hello` function
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(echo))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }

    dotenv().ok();

    let powerwall = PowerwallApi::new()?;

    #[cfg(feature = "i2c_display")]
    // let mut display = i2c_display::RaspiWithDisplay::new();

    let adapter = WS28xxSpiAdapter::new("/dev/spidev0.0").unwrap();
    let seven_segment_display = SevenSegmentDisplayString::new(adapter, 8);
    let mut display = rgbdigit_display::RgbDigitDisplay {
        display: &seven_segment_display,
        solar_generation_status: &mut seven_segment_display.derive_numeric_display(&[4, 5]),
        house_consumption_status: &mut seven_segment_display.derive_numeric_display(&[6,7]),
        battery_status: &mut seven_segment_display.derive_numeric_display(&[0, 1]),
        grid_status: &mut seven_segment_display.derive_numeric_display(&[2, 3]),
        gradient: colorgrad::viridis()
    };

    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_copy = Arc::clone(&shutdown);
    ctrlc::set_handler(move || {
        println!("received ctrl+c");
        shutdown_copy.store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    #[cfg(not(feature = "i2c_display"))]
    let mut display = console_display::ConsoleDisplay {};

    display.startup()?;

    let res = main_loop(powerwall, Box::new(&mut display), shutdown).await;

    // if let Err(e) = res {
    //     display.show_error(&e)?;
    //     return Err(e);
    // } else {
    //     display.shutdown()?;
    // }
    Ok(())
}
