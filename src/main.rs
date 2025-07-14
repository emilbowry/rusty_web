// src/main.rs

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

// 1. Declare the http module. Rust will look for `src/http/mod.rs`.
mod http;

// 2. Import the necessary types from our new module and the standard library.
use http::{HttpRequest, Method, ParseError, Header};

/// The entry point of our server application.
fn main() {
    // Bind to a local address and port.
    let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind to address");
    println!("Server listening on http://127.0.0.1:7878");

    // The `incoming()` method returns an iterator over the connections received.
    // This loop processes each connection sequentially (i.e., it's single-threaded).
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // A new connection has been established.
                println!("\n--- New Connection Accepted ---");
                handle_connection(stream);
            }
            Err(e) => {
                // A connection attempt failed.
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

/// Orchestrates the handling of a single TCP connection.
/// Its job is to read data, invoke the parser, and delegate to the router or an error handler.
fn handle_connection(mut stream: TcpStream) {
    // A buffer to hold the raw request data from the client.
    // 2048 bytes is a reasonable size for simple requests.
    let mut buffer = [0; 2048];
    
    // A buffer that the low-level parser will use to store slices of header data.
    // This avoids allocating memory for headers during the initial parse.
    let mut headers = [Header { name: "", value: &[] }; 32];

    // Read data from the TCP stream into our buffer.
    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => {
            println!("Client disconnected gracefully.");
            return;
        },
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read from stream: {}", e);
            return;
        }
    };

    println!("Received {} bytes of data.", bytes_read);

    // Attempt to parse the raw bytes into a low-level, borrowed request.
    match http::parse_request(&buffer[..bytes_read], &mut headers) {
        Ok((borrowed_request, _)) => {
            // The low-level parse was successful. Now, convert it to our high-level, owned HttpRequest.
            // This step validates the method and copies the data so it's easier to work with.
            match HttpRequest::try_from(borrowed_request) {
                Ok(request) => {
                    // The request is fully parsed and validated. Pass it to the router.
                    println!("Successfully parsed request: {} {}", request.method, request.path);
                    route_request(request, &mut stream);
                }
                Err(e) => {
                    // The request was valid HTTP, but we don't support something in it.
                    eprintln!("Failed to process request: {:?}", e);
                    send_error_response(&mut stream, "400 Bad Request", "Bad Request");
                }
            }
        }
        Err(e) => {
            // The raw bytes were not a valid HTTP request.
            eprintln!("Failed to parse request: {:?}", e);
            send_error_response(&mut stream, "400 Bad Request", "Bad Request");
        }
    }
}

/// Contains the application logic. It inspects the request and decides what response to send.
fn route_request(request: HttpRequest, stream: &mut TcpStream) {
    // Use a `match` statement to elegantly handle different routes and methods.
    match (&request.method, request.path.as_str()) {
        // Route for GET requests to the root path "/".
        (Method::Get, "/") => {
            let response_body = "<h1>Welcome to the Robust Rusty Web Server!</h1><p>This is served from the new router.</p>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).unwrap_or_else(|e| eprintln!("Failed to write response: {}", e));
        }

        // Add your JSON API endpoint here!
        (Method::Get, "/api/message") => {
            // Note: We'll need to add serde_json to Cargo.toml to make this work.
            let response_body = r#"{"status":"success","message":"Hello from the API!"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).unwrap_or_else(|e| eprintln!("Failed to write response: {}", e));
        }

        // A catch-all for any other path or method, resulting in a 404.
        _ => {
            send_error_response(stream, "404 Not Found", "Not Found");
        }
    }
    stream.flush().unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
}

/// A reusable helper function to send a standardized HTTP error response.
fn send_error_response(stream: &mut TcpStream, status_line: &str, message: &str) {
    let response_body = format!("<h1>{}</h1><p>{}</p>", status_line, message);
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        response_body.len(),
        response_body
    );
    stream.write_all(response.as_bytes()).unwrap_or_else(|e| eprintln!("Failed to write error response: {}", e));
    stream.flush().unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
}