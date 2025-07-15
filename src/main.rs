// src/main.rs

use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};

mod http;

use http::{HttpRequest, Method, Header, Response};

fn main() {
    // let listener = TcpListener::bind("127.0.0.1:7878").expect("Failed to bind to address");
    let listener = TcpListener::bind("0.0.0.0:7878").expect("Failed to bind to address");
    println!("Server listening on http://0.0.0.0:7878");

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

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 2048];
    let mut headers = [Header { name: "", value: &[] }; 32];

    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => { println!("Client disconnected gracefully."); return; },
        Ok(n) => n,
        Err(e) => { eprintln!("Failed to read from stream: {}", e); return; }
    };

    println!("Received {} bytes of data.", bytes_read);

    let response = match http::parse_request(&buffer[..bytes_read], &mut headers) {
        Ok((borrowed_request, _)) => {
            match HttpRequest::try_from(borrowed_request) {
                Ok(request) => {
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

    stream.write_all(&response.into_bytes()).unwrap_or_else(|e| eprintln!("Failed to write response: {}", e));
    stream.flush().unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
}


fn add_cors_headers(request: HttpRequest) -> Response {
    let mut response = log_request(request);

    response.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    
    // Return the modified response.
    response
}

fn log_request(request: HttpRequest) -> Response {
    let method = request.method.clone(); 
    let path = request.path.clone();

    let response = route_request(request);

    println!("-> Request: {:?} {} -> Response: {} {}", method, path, response.status_code, response.status_text);

    response
}

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

        _ => {
            Response::not_found()
        }
    }
}