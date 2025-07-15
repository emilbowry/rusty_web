// src/main.rs

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod http;
use http::{HttpRequest, Method, Header, Response};

#[tokio::main]
async fn main() {
    // --- THIS IS THE LINE TO CHANGE ---
    // Bind to 0.0.0.0 to accept connections from outside the VM (via port forwarding).
    let listener = TcpListener::bind("0.0.0.0:7878").await.expect("Failed to bind to address");
    
    println!("Async Server listening on 0.0.0.0:7878");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                println!("\n--- New Connection Accepted ---");
                tokio::spawn(async move {
                    handle_connection(stream).await;
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream) {
    // let mut buffer = [0; 2048];
    let mut buffer = [0; 8192];
    let mut headers = [Header { name: "", value: &[] }; 32];

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

    stream.write_all(&response.into_bytes()).await.unwrap_or_else(|e| eprintln!("Failed to write response: {}", e));
    stream.flush().await.unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
}

// --- NO CHANGES TO MIDDLEWARE OR ROUTER ---

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

// In src/main.rs

fn route_request(request: HttpRequest) -> Response {
    match (&request.method, request.path.as_str()) {
        (Method::Get, "/") => {
            let body = "<h1>Welcome!</h1><p>This is the ASYNCHRONOUS Rusty Web server.</p>".as_bytes().to_vec();
            Response::ok(body, "text/html")
        }

        // --- ADD THIS NEW MATCH ARM ---
        // Handle the CORS preflight request for the API endpoint.
        (Method::Options, "/api/message") => {
            let mut res = Response::no_content();
            // Tell the browser which methods and headers are allowed.
            res.headers.insert("Access-Control-Allow-Methods".to_string(), "GET, OPTIONS".to_string());
            res.headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type".to_string());
            res
        }
        // -----------------------------

        (Method::Get, "/api/message") => {
            let body = r#"{"framework":"Rusty Web","status":"async and awesome"}"#.as_bytes().to_vec();
            Response::ok(body, "application/json")
        }
        
        _ => Response::not_found(),
    }
}