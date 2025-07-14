// src/main.rs

use std::net::{TcpListener, TcpStream};
use std::io::{prelude::*, BufReader};
use serde::Serialize;
use serde_json;

// A struct that we can serialize into JSON.
// The `derive(Serialize)` macro does all the hard work for us.
#[derive(Serialize)]
struct Message {
    status: String,
    text: String,
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    println!("Server listening on port 7878...");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    // Use a BufReader to read the request line by line.
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    // Simple "Router" logic
    // We check the first line of the HTTP request.
    // e.g., "GET /api/message HTTP/1.1"
    let (status_line, response_body) = if request_line == "GET / HTTP/1.1" {
        // Handle the root path
        ("HTTP/1.1 200 OK", "<h1>Welcome to Rusty Web</h1>".to_string())
    } else if request_line == "GET /api/message HTTP/1.1" {
        // Handle our API endpoint
        let message = Message {
            status: "success".to_string(),
            text: "This is a platform-agnostic JSON response!".to_string(),
        };
        // Serialize our Message struct into a JSON string.
        let json_body = serde_json::to_string(&message).unwrap();
        ("HTTP/1.1 200 OK", json_body)
    } else {
        // Handle all other paths with a 404
        ("HTTP/1.1 404 NOT FOUND", "<h1>404 Not Found</h1>".to_string())
    };

    // Construct the final response, including headers.
    // Note the Content-Type for JSON!
    let headers = if request_line.contains("/api/") {
        "Content-Type: application/json"
    } else {
        "Content-Type: text/html"
    };

    let response = format!(
        "{}\r\nContent-Length: {}\r\n{}\r\n\r\n{}",
        status_line,
        response_body.len(),
        headers,
        response_body
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}