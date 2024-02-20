use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use regex::Regex;
use crate::parser::parse_url_encoded;

pub type Headers = HashMap<String, Vec<String>>;


#[derive(Debug)]
pub enum RequestHeaderError {
    /// Occurs if header size is larger than the given limit
    MaxSizeExceed,
    /// Occurs if client is disconnected
    ClientDisconnected,
}


/// It will try to read headers from the tcp stream.
/// Returns type `RequestHeaderError` if failed to extract headers.
pub fn extract_headers(stream: &mut TcpStream, start_header: &mut String,
                       partial_body_bytes: &mut Vec<u8>, max_size: usize) -> Result<Headers, RequestHeaderError> {
    let mut header_bytes = Vec::new();

    let mut read_all_headers = false;

    while !read_all_headers {
        if header_bytes.len() > max_size {
            return Err(RequestHeaderError::MaxSizeExceed);
        }

        let mut buffer = [0u8; 1024];
        let read_result = stream.read(&mut buffer);

        let read_size;

        match read_result {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    return Err(RequestHeaderError::ClientDisconnected);
                }
                read_size = bytes_read;
            }

            Err(_) => {
                return Err(RequestHeaderError::ClientDisconnected);
            }
        }

        // There will be index if the header is ended. However, contains_full_header don't take
        // complete request header.
        if let Some(header_end_index) = contains_full_headers(&buffer) {
            header_bytes.extend(&buffer[..header_end_index]);

            // Body starts from header_end_index + "\r\n\r\n"
            partial_body_bytes.extend(&buffer[header_end_index + 4..read_size]);
            read_all_headers = true;
        } else {
            header_bytes.extend(&buffer[..read_size]);
        }
    }

    let raw_request_headers = String::from_utf8(header_bytes)
        .expect("Unsupported header encoding.");
    let mut header_lines = raw_request_headers.split("\r\n");

    *start_header = String::from(header_lines.next().unwrap());

    let mut headers: Headers = HashMap::new();
    for header in header_lines {
        let key_value = parse_header(header);

        if let Some((key, value)) = key_value {
            if headers.contains_key(&key) {
                let values = headers.get_mut(&key).unwrap();
                values.push(value);
            } else {
                let header_value: Vec<String> = vec![value];
                headers.insert(key, header_value);
            }
        }
    };

    return Ok(headers);
}


/// Returns content length from the `Header` if available
pub fn content_length(headers: &Headers) -> Option<usize> {
    if let Some(values) = headers.get("Content-Length") {
        if values.len() > 0 {
            let value = values.get(0).unwrap();
            let content_length_value = value.parse::<usize>().expect("Invalid content length");
            return Some(content_length_value);
        }
    }

    return None;
}


/// Returns the value of `Connection` header if available
pub fn connection_type(headers: &Headers) -> Option<String> {
    if let Some(values) = headers.get("Connection") {
        if values.len() > 0 {
            let value = values.get(0).unwrap();
            return Some(value.to_owned());
        }
    }

    return None;
}

/// Returns `Host` value from the Header if available.
pub fn host(headers: &Headers) -> Option<String> {
    let host = headers.get("Host");
    if let Some(host) = host {
        if host.len() > 0 {
            let value = host.get(0).unwrap();
            return Some(value.to_string());
        }
    }

    return None;
}


/// Returns `Content-Type` value from the header if available
pub fn extract_content_type(headers: &Headers) -> Option<String> {
    if let Some(values) = headers.get("Content-Type") {
        let value = values.get(0).expect("Content-Type implementation error");
        return Some(value.to_owned());
    }

    return None;
}

/// Returns size of header end position if header ends with "\r\n\r\n"
pub fn contains_full_headers(buffer: &[u8]) -> Option<usize> {
    let end_header_bytes = b"\r\n\r\n";
    buffer.windows(end_header_bytes.len()).position(|window| window == end_header_bytes)
}


/// Returns the request method and raw path from the header line if matched
/// ```markdown
/// GET / HTTP/1.1
/// ```
pub fn parse_request_method_header(line: &str) -> Option<(String, String)> {
    let pattern = Regex::new(r"(?<method>.+) (?<path>.+) (.+)").unwrap();

    if let Some(groups) = pattern.captures(line) {
        let request_method = &groups["method"];
        let path = &groups["path"];
        return Some((request_method.to_string(), path.to_string()));
    }

    return None;
}

/// Returns key value pair from the header line
///
/// Input example:
/// ```markdown
/// Content-Length: 10
/// ```
pub fn parse_header(line: &str) -> Option<(String, String)> {
    let header_line: Vec<&str> = line.splitn(2, ":").collect();
    if header_line.len() >= 2 {
        let name = header_line.get(0).unwrap().trim().to_string();
        let value = header_line.get(1).unwrap().trim().to_string();
        return Some((name, value));
    }
    return None;
}


/// Returns map of url encoded key values
/// Example: `/search?name=John&age=22`
pub fn query_params_from_raw(raw_path: &String) -> HashMap<String, Vec<String>> {
    let query_params: HashMap<String, Vec<String>> = HashMap::new();
    let match_result = raw_path.find("?");

    if !match_result.is_some() {
        return query_params;
    }

    let index = match_result.unwrap();
    if index == raw_path.len() - 1 {
        return query_params;
    }

    let slice = &raw_path[index + 1..raw_path.len()];
    return parse_url_encoded(slice);
}
