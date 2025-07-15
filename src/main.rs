// src/main.rs

// --- NEW IMPORTS ---
// We now use Tokio's I/O types and traits.
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// 1. Declare the http module (no change here)
mod http;

// 2. Import our HTTP types (no change here)
use http::{HttpRequest, Method, Header, Response};

/// The entry point of our server application.
/// The `#[tokio::main]` macro sets up the asynchronous runtime.
#[tokio::main]
async fn main() {
    // Bind the listener to the address. Note that this is now `tokio::net::TcpListener`.
    let listener = TcpListener::bind("127.0.0.1:7878").await.expect("Failed to bind to address");
    println!("Async Server listening on http://127.0.0.1:7878");

    // The main server loop.
    loop {
        // Asynchronously wait for an inbound connection.
        // `accept()` returns a tuple of `(socket, address)`.
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("\n--- New Connection Accepted ---");
                // A new connection has been established.
                // Spawn a new asynchronous task to handle this connection.
                // The `move` keyword transfers ownership of the `stream` to the new task.
                tokio::spawn(async move {
                    handle_connection(stream).await;
                });
            }
            Err(e) => {
                // A connection attempt failed.
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

/// Handles a single connection. The function is now `async`.
/// It takes a `tokio::net::TcpStream` instead of a `std::net::TcpStream`.
async fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 2048];
    let mut headers = [Header { name: "", value: &[] }; 32];

    // Asynchronously read data from the stream.
    let bytes_read = match stream.read(&mut buffer).await {
        Ok(0) => { println!("Client disconnected gracefully."); return; },
        Ok(n) => n,
        Err(e) => { eprintln!("Failed to read from stream: {}", e); return; }
    };

    println!("Received {} bytes of data.", bytes_read);

    let response = match http::parse_request(&buffer[..bytes_read], &mut headers) {
        Ok((borrowed_request, _)) => {
            match HttpRequest::try_from(borrowed_request) {
                Ok(request) => add_cors_headers(request),
                Err(e) => {
                    eprintln!("Failed to process request: {:?}", e);
                    Response::bad_request()
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse request: {:?}", e);
            Response::bad_request()
        }
    };

    // Asynchronously write the final, serialized response to the stream.
    stream.write_all(&response.into_bytes()).await.unwrap_or_else(|e| eprintln!("Failed to write response: {}", e));
    // Asynchronously flush the stream.
    stream.flush().await.unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
}

// --- NO CHANGES BELOW THIS LINE ---
// These functions are CPU-bound and do not need to be async.

// --- MIDDLEWARE LAYER ---

fn add_cors_headers(request: HttpRequest) -> Response {
    let mut response = log_request(request);
    response.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    response
}

fn log_request(request: HttpRequest) -> Response {
    let method = request.method.clone();
    let path = request.path.clone();
    let response = route_request(request);
    println!("-> Request: {:?} {} -> Response: {} {}", method, path, response.status_code, response.status_text);
    response
}

// --- ROUTER / HANDLER LAYER ---

fn route_request(request: HttpRequest) -> Response {
    match (&request.method, request.path.as_str()) {
        (Method::Get, "/") => {
            let body = "<h1>Welcome!</h1><p>This is the ASYNCHRONOUS Rusty Web server.</p>".as_bytes().to_vec();
            Response::ok(body, "text/html")
        }
        (Method::Get, "/api/message") => {
            let body = r#"{"framework":"Rusty Web","status":"async and awesome"}"#.as_bytes().to_vec();
            Response::ok(body, "application/json")
        }
        _ => Response::not_found(),
    }
}