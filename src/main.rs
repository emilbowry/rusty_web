// src/main.rs

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

// 1. Declare the http module
mod http;

// 2. Import the necessary types. We now need `Response`.
use http::{HttpRequest, Method, Header, Response};

/// The entry point of our server application.
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind to address");
    println!("Server listening on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("\n--- New Connection Accepted ---");
                handle_connection(stream);
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

/// Orchestrates handling a single connection.
/// Its responsibility is now to parse the request, pass it through the middleware chain,
/// and write the final response to the stream.
fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 2048];
    let mut headers = [Header { name: "", value: &[] }; 32];

    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => { println!("Client disconnected gracefully."); return; },
        Ok(n) => n,
        Err(e) => { eprintln!("Failed to read from stream: {}", e); return; }
    };

    println!("Received {} bytes of data.", bytes_read);

    // The final response that will be sent to the client.
    let response = match http::parse_request(&buffer[..bytes_read], &mut headers) {
        Ok((borrowed_request, _)) => {
            match HttpRequest::try_from(borrowed_request) {
                Ok(request) => {
                    // The request is valid. Pass it to the outermost middleware.
                    add_cors_headers(request)
                }
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

    // Write the final, serialized response to the stream. This is the ONLY write point.
    stream.write_all(&response.into_bytes()).unwrap_or_else(|e| eprintln!("Failed to write response: {}", e));
    stream.flush().unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
}

// --- MIDDLEWARE LAYER ---

/// Middleware #1: Adds CORS headers to the response.
/// This is the outermost layer. It calls the next middleware in the chain.
fn add_cors_headers(request: HttpRequest) -> Response {
    // Pass the request to the next layer to get a response.
    let mut response = log_request(request);

    // Modify the response by adding the CORS header.
    // The "*" allows any origin, which is convenient for local development.
    response.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    
    // Return the modified response.
    response
}

/// Middleware #2: Logs the request and the resulting response status.
/// This is the inner layer. It calls the final router/handler.
fn log_request(request: HttpRequest) -> Response {
    let method = request.method.clone(); // Clone method and path for logging
    let path = request.path.clone();

    // Pass the request to the final handler to get the response.
    let response = route_request(request);

    // Log the details after the response has been generated.
    println!("-> Request: {:?} {} -> Response: {} {}", method, path, response.status_code, response.status_text);

    // Return the response unmodified.
    response
}

// --- ROUTER / HANDLER LAYER ---

/// The core application logic. It matches the request to a specific action.
/// Its only job is to produce a `Response` object.
fn route_request(request: HttpRequest) -> Response {
    match (&request.method, request.path.as_str()) {
        (Method::Get, "/") => {
            let body = "<h1>Welcome!</h1><p>This is the middleware-powered Rusty Web server.</p>".as_bytes().to_vec();
            Response::ok(body, "text/html")
        }

        (Method::Get, "/api/message") => {
            // Using a raw string for JSON. For complex objects, `serde_json::to_vec` would be better.
            let body = r#"{"framework":"Rusty Web","status":"awesome"}"#.as_bytes().to_vec();
            Response::ok(body, "application/json")
        }

        // Catch-all for any other route.
        _ => {
            Response::not_found()
        }
    }
}