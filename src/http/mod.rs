use std::str;
use std::collections::HashMap; 


#[derive(Debug, PartialEq)]
pub enum ParseError {
    Partial,
    InvalidMethod,
    InvalidPath,
    InvalidVersion,
    InvalidHeader,
    TooManyHeaders,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Header<'buf> {
    pub name: &'buf str,
    pub value: &'buf [u8],
}

#[derive(Debug, PartialEq)]
pub struct Request<'buf, 'h> {
    pub method: &'buf str,
    pub path: &'buf str,
    pub version: &'buf str,
    pub headers: &'h [Header<'buf>],
    pub body: &'buf [u8],
}

const MAX_HEADERS: usize = 32;

pub fn parse_request<'buf, 'h>(
    buffer: &'buf [u8],
    headers: &'h mut [Header<'buf>],
) -> Result<(Request<'buf, 'h>, usize), ParseError> {
    let mut cursor = 0;

    let request_line_end = find_crlf(&buffer[cursor..]).ok_or(ParseError::Partial)?;
    let request_line_bytes = &buffer[cursor..cursor + request_line_end];
    let mut parts = request_line_bytes.split(|&b| b == b' ');

    let method_bytes = parts.next().filter(|s| !s.is_empty()).ok_or(ParseError::InvalidMethod)?;
    let path_bytes = parts.next().filter(|s| !s.is_empty()).ok_or(ParseError::InvalidPath)?;
    let version_bytes = parts.next().filter(|s| !s.is_empty()).ok_or(ParseError::InvalidVersion)?;
    
    cursor += request_line_end + 2;

    let mut header_count = 0;
    loop {
        let header_line_end = find_crlf(&buffer[cursor..]).ok_or(ParseError::Partial)?;
        if header_line_end == 0 {
            cursor += 2;
            break;
        }
        if header_count >= headers.len() {
            return Err(ParseError::TooManyHeaders);
        }
        let header_line = &buffer[cursor..cursor + header_line_end];
        let colon_pos = header_line.iter().position(|&b| b == b':').ok_or(ParseError::InvalidHeader)?;
        
        let name = str::from_utf8(&header_line[..colon_pos]).map_err(|_| ParseError::InvalidHeader)?;
        let value_start = colon_pos + 1;
        let value = &header_line[value_start..].trim_start();
        
        headers[header_count] = Header { name, value };
        header_count += 1;
        cursor += header_line_end + 2;
    }
    
    let parsed_headers = &headers[..header_count];

    let mut content_length = 0;
    for header in parsed_headers {
        if header.name.eq_ignore_ascii_case("Content-Length") {
            let value_str = str::from_utf8(header.value).map_err(|_| ParseError::InvalidHeader)?;
            content_length = value_str.parse::<usize>().map_err(|_| ParseError::InvalidHeader)?;
            break;
        }
    }

    // The body starts exactly where the cursor was left after the header loop.
    let body_start = cursor;
    let total_request_size = body_start + content_length;

    if buffer.len() < total_request_size {
        return Err(ParseError::Partial);
    }

    let body = &buffer[body_start..total_request_size];

    let request = Request {
        method: str::from_utf8(method_bytes).map_err(|_| ParseError::InvalidMethod)?,
        path: str::from_utf8(path_bytes).map_err(|_| ParseError::InvalidPath)?,
        version: str::from_utf8(version_bytes).map_err(|_| ParseError::InvalidVersion)?,
        headers: parsed_headers,
        body,
    };
    
    Ok((request, total_request_size))
}
fn find_crlf(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\r\n")
}

trait TrimStart {
    fn trim_start(&self) -> &Self;
}

impl TrimStart for [u8] {
    fn trim_start(&self) -> &Self {
        if let Some(pos) = self.iter().position(|&b| b != b' ' && b != b'\t') {
            &self[pos..]
        } else {
            &[]
        }
    }
}

// --- NEW: Application-Level (Owned) HTTP Types ---

#[derive(Debug, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Patch,
    Trace,
}

#[derive(Debug, PartialEq)]
pub struct HttpRequest {
    pub method: Method,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, Vec<u8>>,
    pub body: Vec<u8>,
}

impl<'buf, 'h> TryFrom<Request<'buf, 'h>> for HttpRequest {
    type Error = ParseError;

    fn try_from(borrowed_req: Request<'buf, 'h>) -> Result<Self, Self::Error> {
        let method = match borrowed_req.method {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            "CONNECT" => Method::Connect,
            "PATCH" => Method::Patch,
            "TRACE" => Method::Trace,
            _ => return Err(ParseError::InvalidMethod),
        };

        let mut headers = HashMap::new();
        for header in borrowed_req.headers {
            headers.insert(header.name.to_lowercase(), header.value.to_vec());
        }

        Ok(HttpRequest {
            method,
            path: borrowed_req.path.to_string(),
            version: borrowed_req.version.to_string(),
            headers,
            body: borrowed_req.body.to_vec(),
        })
    }
}


// fn run_test(test_name: &str, request_bytes: &[u8], header_buf_size: usize) {
//     println!("--- Running Test: {} ---", test_name);
    
//     let mut headers = vec![Header { name: "", value: &[] }; header_buf_size];

//     match parse_request(request_bytes, &mut headers) {
//         Ok((request, bytes_consumed)) => {
//             println!("✅ Success! Parsed {} bytes.", bytes_consumed);
//             println!("{:#?}\n", request);
//         }
//         Err(e) => {
//             println!("✅ Success! Correctly failed with error: {:?}\n", e);
//         }
//     }
// }


// fn main() {
//     println!("--- Running Low-Level Parser Test Suite ---\n");

//     // --- All original tests remain the same ---
//     run_test("Passing Case (GET with headers)", b"GET /test HTTP/1.1\r\nHost: example.com\r\nConnection: keep-alive\r\n\r\n", MAX_HEADERS);
//     run_test("Error Case: Partial (incomplete request)", b"GET /test HTTP/1.1\r\nHost: examp", MAX_HEADERS);
//     run_test("Error Case: InvalidMethod (no method)", b" /test HTTP/1.1\r\n\r\n", MAX_HEADERS);
//     run_test("Error Case: InvalidPath (no path)", b"GET \r\n\r\n", MAX_HEADERS);
//     run_test("Error Case: InvalidVersion (no version)", b"GET /test\r\n\r\n", MAX_HEADERS);
//     run_test("Error Case: InvalidHeader (no colon)", b"GET /test HTTP/1.1\r\nHost example.com\r\n\r\n", MAX_HEADERS);
//     run_test("Error Case: TooManyHeaders", b"GET /test HTTP/1.1\r\nHeader1: a\r\nHeader2: b\r\nHeader3: c\r\n\r\n", 2);

//     println!("\n--- Running High-Level Conversion Test Suite ---\n");

//     // --- Test 8: High-Level Conversion (POST with body) ---
//     println!("--- Running Test: High-Level Conversion (POST with body) ---");
    
//     // THE FIX: Content-Length is now correctly set to 17.
//     let post_req_bytes = b"POST /api/users HTTP/1.1\r\n\
// Content-Type: application/json\r\n\
// Content-Length: 17\r\n\
// Host: localhost\r\n\
// \r\n\
// {\"user\": \"alice\"}";

//     let mut headers = vec![Header { name: "", value: &[] }; MAX_HEADERS];
    
//     match parse_request(post_req_bytes, &mut headers) {
//         Ok((parsed_req, _)) => {
//             println!("✅ Low-level parser succeeded.");
            
//             match HttpRequest::try_from(parsed_req) {
//                 Ok(http_request) => {
//                     println!("✅ High-level conversion succeeded!");
//                     println!("{:#?}\n", http_request);

//                     // Assertions now check for the correct values.
//                     assert_eq!(http_request.method, Method::Post);
//                     assert_eq!(http_request.path, "/api/users");
//                     assert_eq!(http_request.headers.get("content-type").unwrap(), b"application/json");
//                     assert_eq!(http_request.headers.get("content-length").unwrap(), b"17");
//                     assert_eq!(http_request.body, b"{\"user\": \"alice\"}");
//                 }
//                 Err(e) => {
//                     println!("❌ High-level conversion failed with error: {:?}", e);
//                 }
//             }
//         }
//         Err(e) => {
//             println!("❌ Low-level parser failed with error: {:?}", e);
//         }
//     }
// }

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl Response {
    /// Creates a new Response with a status code, text, and optional body.
    pub fn new(status_code: u16, status_text: String, body: Option<Vec<u8>>) -> Self {
        Response {
            status_code,
            status_text,
            headers: HashMap::new(),
            body,
        }
    }

    /// Helper to create a `200 OK` response with a given body and content type.
    pub fn ok(body: Vec<u8>, content_type: &str) -> Self {
        let mut res = Response::new(200, "OK".to_string(), Some(body));
        res.headers.insert("Content-Type".to_string(), content_type.to_string());
        res
    }

    /// Helper to create a standard `404 Not Found` response.
    pub fn not_found() -> Self {
        let body = "<h1>404 Not Found</h1>".as_bytes().to_vec();
        let mut res = Response::new(404, "Not Found".to_string(), Some(body));
        res.headers.insert("Content-Type".to_string(), "text/html".to_string());
        res
    }

    /// Helper to create a standard `400 Bad Request` response.
    pub fn bad_request() -> Self {
        let body = "<h1>400 Bad Request</h1>".as_bytes().to_vec();
        let mut res = Response::new(400, "Bad Request".to_string(), Some(body));
        res.headers.insert("Content-Type".to_string(), "text/html".to_string());
        res
    }

    /// Serializes the Response struct into a `Vec<u8>` of raw HTTP response bytes.
    pub fn into_bytes(&self) -> Vec<u8> {
        // Start with the status line, e.g., "HTTP/1.1 200 OK\r\n"
        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.status_text);

        // Create a mutable string for headers
        let mut headers_str = String::new();

        // Add all headers from the HashMap
        for (name, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", name, value));
        }
        
        // Automatically calculate and add the Content-Length header based on the body's size.
        let content_length = self.body.as_ref().map_or(0, |b| b.len());
        headers_str.push_str(&format!("Content-Length: {}\r\n", content_length));

        // Combine the status line, headers, the final CRLF, and the body.
        let mut response_bytes = Vec::new();
        response_bytes.extend_from_slice(status_line.as_bytes());
        response_bytes.extend_from_slice(headers_str.as_bytes());
        response_bytes.extend_from_slice(b"\r\n"); // The empty line separating headers from body

        if let Some(body) = &self.body {
            response_bytes.extend_from_slice(body);
        }

        response_bytes
    }
}