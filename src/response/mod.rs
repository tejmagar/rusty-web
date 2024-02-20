use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::net::{Shutdown};
use crate::headers::Headers;
use crate::request::Request;
use crate::status::{Status, StatusCode, StatusMethods};

pub struct Response {
    pub request: Request,
    // Response headers
    pub headers: Option<Headers>,
    pub status: Option<usize>,
    pub fixed_content: Option<String>,
}

impl Response {
    pub fn new(request: Request) -> Self {
        return Self {
            request,
            headers: None,
            status: None,
            fixed_content: None,
        };
    }

    fn init_headers(&mut self) {
        if !self.headers.is_some() {
            self.headers = Some(HashMap::new());
        }
    }

    pub fn set_content_type(&mut self, text: &str) {
        self.init_headers();

        // Unwrap header hashmap
        if let Some(ref mut headers) = self.headers {
            if headers.contains_key("Content-Type") {
                let content_types = headers.get_mut("Content-Type").unwrap();

                // Content type can only have one instance
                content_types[0] = text.to_string();
            } else {
                let content_types = vec![text.to_string()];
                let _ = headers.insert("Content-Type".to_string(), content_types);
            }
        }
    }

    /// Headers will be keep appending to the list if already exists
    pub fn add_header(&mut self, name: &str, value: &str) -> &mut Self {
        self.init_headers();

        // Unwrap header hashmap
        if let Some(ref mut headers) = self.headers {
            if headers.contains_key(name) {
                let values = headers.get_mut(name).unwrap();
                values.push(value.to_string());
            } else {
                let values = vec![value.to_string()];
                headers.insert(name.to_string(), values);
            }
        }

        return self;
    }

    pub fn html<T: StatusCode>(&mut self, status: T, text: String) -> &mut Self {
        self.set_content(status.to_usize(), text);
        self.set_content_type("text/html");
        return self;
    }

    pub fn json<T: StatusCode>(&mut self, status: T, text: String) -> &mut Self {
        self.set_content(status.to_usize(), text);
        self.set_content_type("application/json");
        return self;
    }

    pub fn set_content(&mut self, status: usize, text: String) -> &mut Self {
        self.status = Some(status);
        self.fixed_content = Some(text);
        return self;
    }

    pub fn send(&mut self) {
        if self.status.is_some() {
            let request = &self.request;
            let access_from: String;

            match request.stream.peer_addr() {
                Ok(addr) => {
                    access_from = addr.to_string();
                }

                Err(_) => {
                    access_from = "UnKnown".to_string()
                }
            }

            println!("{} - \"{} {} {}\"", access_from, request.method, request.pathname,
                     self.status.unwrap());
            self.write_http();
        }
    }

    fn write_http(&mut self) {
        let should_close = self.request.should_close_connection();

        let headers = self.headers.as_mut().expect("Response headers missing.");
        let content_length = format!("{}", self.fixed_content.as_ref()
            .expect("Fixed content is missing.").len());
        headers.insert("Content-Length".to_string(), vec![content_length]);

        if !should_close {
            headers.insert("Connection".to_string(), vec!["keep-alive".to_string()]);
        }

        // Write repose headers
        let headers = self.prepare_raw_headers();

        let cloned_stream = self.request.stream.try_clone();
        if !cloned_stream.is_ok() {
            println!("Connection closed");
            self.request.context.dont_wait();
            return;
        }

        let mut buf_writer = BufWriter::new(cloned_stream.unwrap());
        match buf_writer.write_all(headers.as_bytes()) {
            Ok(_) => {}
            Err(_) => {
                println!("Connection closed");
                self.request.context.dont_wait();
                return;
            }
        }

        // Write response body
        if self.request.method != "HEAD" {
            if let Some(content) = &self.fixed_content {
                buf_writer.write_all(content.as_bytes()).unwrap();
            }
        }

        // Flush the buffer
        if !buf_writer.flush().is_ok() {
            print!("Connection closed");
            self.request.context.dont_wait();
        };

        if should_close {
            let _ = self.request.stream.shutdown(Shutdown::Both);
            self.request.context.dont_wait();
        }
    }

    fn prepare_raw_headers(&mut self) -> String {
        let status_code = self.status.expect("Status code not set.");

        let mut status_text = Status::status_text(status_code);
        if !status_text.is_some() {
            status_text = Some("Custom Status".to_string());
        }

        // Header start
        let mut raw_headers = format!("HTTP/1.1 {} {}\r\n", self.status.unwrap(), status_text.unwrap());

        if let Some(headers) = &self.headers {
            for header_name in headers.keys() {
                let values = headers.get(header_name).unwrap();

                for value in values {
                    let header_line = format!("{}: {}\r\n", header_name, value);
                    raw_headers.push_str(&header_line);
                }
            }
        }

        // Header end
        raw_headers.push_str("\r\n");
        return raw_headers;
    }
}
