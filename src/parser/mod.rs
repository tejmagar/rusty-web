use std::collections::HashMap;

pub mod body {
    use std::io::{Seek, SeekFrom, Write};
    use tempfile::NamedTempFile;
    use crate::headers;
    use crate::headers::Headers;
    use crate::parser::body::reader::StreamReader;

    pub struct Limits {
        pub max_body_size: usize,
    }

    #[derive(Debug)]
    pub enum BodyReadError {
        MaxBodySizeExceed,
        ContentLengthMissing,
        BodyAlreadyRead,
        Others(&'static str),
    }

    pub mod reader {
        use std::io::Read;
        use std::net::TcpStream;
        use crate::parser::body::{BodyReadError, Limits};

        pub trait StreamReader {
            fn get_chunk(&mut self) -> Result<Vec<u8>, BodyReadError>;
            fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, BodyReadError>;
        }

        pub struct BodyReader {
            stream: TcpStream,
            content_length: usize,
            bytes_read: usize,
            limits: Limits,
        }

        impl BodyReader {
            pub fn new(stream: TcpStream, content_length: usize, bytes_read: usize, limits: Limits) -> Self {
                println!("{:?}", bytes_read);

                return Self {
                    stream,
                    content_length,
                    bytes_read,
                    limits,
                };
            }
        }

        impl StreamReader for BodyReader {
            fn get_chunk(&mut self) -> Result<Vec<u8>, BodyReadError> {
                if self.bytes_read >= self.content_length {
                    return Err(BodyReadError::MaxBodySizeExceed);
                }

                if self.bytes_read >= self.limits.max_body_size {
                    return Err(BodyReadError::BodyAlreadyRead);
                }

                let mut buffer = [0u8; 1024];
                let read_result = self.stream.read(&mut buffer);
                if !read_result.is_ok() {
                    return Err(BodyReadError::Others(
                        "Unable to read stream. May be client disconnected."
                    ));
                }

                let chunk_length = read_result.unwrap();
                let chunk = Vec::from(&buffer[0..chunk_length]);
                self.bytes_read += chunk_length;
                return Ok(chunk);
            }

            fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, BodyReadError> {
                if self.bytes_read >= self.content_length {
                    return Err(BodyReadError::MaxBodySizeExceed);
                }

                if self.bytes_read >= self.limits.max_body_size {
                    return Err(BodyReadError::BodyAlreadyRead);
                }

                let mut buffer = vec![0u8; size];
                let read_result = self.stream.read_exact(&mut buffer);
                if !read_result.is_ok() {
                    return Err(BodyReadError::Others(
                        "Unable to read stream. May be client disconnected."
                    ));
                }
                self.bytes_read += size;
                return Ok(buffer);
            }
        }
    }

    pub fn parse<T: StreamReader>(partial_bytes: Vec<u8>, headers: &Headers, mut reader: T)
                                  -> Result<NamedTempFile, BodyReadError> {
        let mut body_buffer = Vec::from(partial_bytes);
        let mut body_read = body_buffer.len();

        let content_length = headers::content_length(&headers);
        if !content_length.is_some() {
            return Err(BodyReadError::ContentLengthMissing);
        }

        // Create new tmp directory
        let temp_file_create = NamedTempFile::new();
        let mut temp_file;

        match temp_file_create {
            Ok(file) => {
                temp_file = file;
            }

            Err(_) => {
                return Err(BodyReadError::Others("Error creating temporary file"));
            }
        }

        let content_length = content_length.unwrap();

        loop {
            let write_result = temp_file.write_all(&body_buffer);
            if !write_result.is_ok() {
                return Err(BodyReadError::Others("Error writing to temporary file"));
            }

            if body_read >= content_length {
                let seek_result = temp_file.seek(SeekFrom::Start(0));
                if !seek_result.is_ok() {
                    return Err(BodyReadError::Others("Failed to seek temporary file"));
                }
                return Ok(temp_file);
            }

            body_buffer.clear();

            let read_result = reader.get_chunk();
            match read_result {
                Ok(chunk) => {
                    body_read += chunk.len();
                    body_buffer.extend(chunk);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }
    }
}

pub fn parse_url_encoded(text: &str) -> HashMap<String, Vec<String>> {
    let mut params = HashMap::new();
    let values = text.split("&");

    for value in values {
        let key_values: Vec<&str> = value.split("=").collect();
        if key_values.len() >= 2 {
            let name = key_values.get(0).unwrap();
            let value = key_values.get(1).unwrap();

            let name_formatted = url_decode(name);
            let value_formatted = url_decode(value);

            if !params.contains_key(&name_formatted) {
                params.insert(name.to_string(), Vec::new());
            }

            let values = params.get_mut(&name_formatted).unwrap();
            values.push(value_formatted);
        }
    }
    return params;
}

pub fn url_decode(value: &str) -> String {
    return match urlencoding::decode(value) {
        Ok(decoded_value) => {
            decoded_value.to_string()
        }
        Err(_) => {
            value.to_string()
        }
    };
}

pub mod url_encoded {
    use std::collections::HashMap;
    use crate::headers;
    use crate::headers::{Headers};
    use crate::parser::parse_url_encoded;
    use crate::parser::url_encoded::reader::StreamReader;

    #[derive(Debug)]
    pub enum UrlEncodedFormDataError {
        /// Occurs, if the request body is not x-www-form-urlencoded
        InvalidFormat(&'static str),
        /// Occurs, if there is no Content-Length header
        ContentLengthMissing(&'static str),
        /// Occurs, if failed to parse the request
        ParsingError(&'static str),
        /// Occurs, if the request body size exceed the given size
        MaxBodySizeExceed(&'static str),
        /// Occurs, if parser requires more data to parse fully, but there is no more data left
        BodyReadEnd,
        /// Occurs, if error not fulfilled by above conditions
        Others(&'static str),
    }

    pub mod reader {
        use std::io::Read;
        use std::net::TcpStream;
        use crate::parser::url_encoded::UrlEncodedFormDataError;

        /// The reusable trait for fetching "x-www-form-urlencoded" form data
        pub trait StreamReader {
            fn get_chunk(&mut self) -> Result<Vec<u8>, UrlEncodedFormDataError>;
            fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, UrlEncodedFormDataError>;
        }

        pub struct UrlEncodedReader {
            pub stream: TcpStream,
            pub content_length: usize,
            // Size of bytes that has been already read
            pub bytes_read: usize,
            pub body_ended: bool,
        }

        impl UrlEncodedReader {
            pub fn new(stream: TcpStream, content_length: usize, bytes_read: usize) -> Self {
                let body_ended;

                if bytes_read == content_length {
                    body_ended = true;
                } else {
                    body_ended = false;
                };

                return Self {
                    stream,
                    content_length,
                    bytes_read,
                    body_ended,
                };
            }

            fn update_read_status(&mut self, new_chunk: &[u8]) {
                self.bytes_read += new_chunk.len();

                if self.bytes_read >= self.content_length {
                    self.body_ended = true;
                }
            }
        }

        impl StreamReader for UrlEncodedReader {
            fn get_chunk(&mut self) -> Result<Vec<u8>, UrlEncodedFormDataError> {
                if self.body_ended {
                    return Err(UrlEncodedFormDataError::BodyReadEnd);
                }

                let mut buffer = [0u8; 1024];
                let read_result = self.stream.read(&mut buffer);

                if !read_result.is_ok() {
                    return Err(UrlEncodedFormDataError::Others(
                        "Unable to read stream. May be client disconnected."
                    ));
                }

                let read_size = read_result.unwrap();

                if read_size == 0 {
                    return Err(UrlEncodedFormDataError::Others("Bytes read size is 0. Probably client disconnected."));
                }

                let chunk = &buffer[0..read_size];
                self.update_read_status(chunk);
                return Ok(chunk.to_vec());
            }

            fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, UrlEncodedFormDataError> {
                if self.body_ended {
                    return Err(UrlEncodedFormDataError::BodyReadEnd);
                }

                let mut buffer = vec![0u8; size];
                let result = self.stream.read_exact(&mut buffer);
                if !result.is_ok() {
                    return Err(UrlEncodedFormDataError::Others(
                        "Unable to read stream. May be client disconnected."
                    ));
                }

                return Ok(buffer.to_vec());
            }
        }
    }

    pub struct Limits {
        pub max_body_size: usize,
    }

    pub type FormFields = HashMap<String, Vec<String>>;

    pub fn parse<T: StreamReader>(partial_bytes: Vec<u8>, headers: &Headers, reader: &mut T,
                                  limits: Limits) -> Result<FormFields, UrlEncodedFormDataError> {
        let mut body_buffer = Vec::from(partial_bytes);
        let content_length = headers::content_length(headers);

        if let Some(content_length) = content_length {
            if content_length > limits.max_body_size {
                return Err(UrlEncodedFormDataError::MaxBodySizeExceed(
                    "Request body size is larger than the limit."
                ));
            }
        } else {
            return Err(UrlEncodedFormDataError::ContentLengthMissing(
                "Content-Length header is missing."
            ));
        }

        let content_length = content_length.unwrap();
        let bytes_read = body_buffer.len();

        // Load all the request body to memory
        while content_length > bytes_read {
            let request_chunk = reader.get_chunk();

            match request_chunk {
                Ok(chunk) => {
                    body_buffer.extend(chunk);
                }

                Err(error) => {
                    return Err(error);
                }
            }
        };

        let value = String::from_utf8_lossy(&body_buffer).to_string();
        let form_values = parse_url_encoded(value.as_str());
        return Ok(form_values);
    }
}

pub mod multipart {
    use std::collections::HashMap;
    use std::io::{Seek, SeekFrom, Write};
    use regex::Regex;
    use tempfile::NamedTempFile;
    use crate::headers;
    use crate::headers::Headers;

    #[derive(Debug)]
    pub enum MultipartFormDataError {
        /// Occurs, if the request body is not multipart/form-data
        InvalidMultiPart(&'static str),
        /// Occurs, if failed to parse the request
        ParsingError(&'static str),
        /// Occurs, if the size of the form part header exceeds the given size
        HeaderSizeExceed(&'static str),
        /// Occurs, if the request body size exceed the given size
        MaxBodySizeExceed(&'static str),
        /// Occurs, if the form part content size exceed
        MaxFieldSizeExceed(String, &'static str),
        /// Occurs, if parser requires more data to parse fully, but there is no more data left
        BodyReadEnd,
        /// Occurs, if error not fulfilled by above conditions
        Others(&'static str),
    }


    /// The reusable trait for fetching multipart bytes
    pub trait StreamReader {
        fn get_chunk(&mut self) -> Result<Vec<u8>, MultipartFormDataError>;
        fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, MultipartFormDataError>;
    }

    /// Extracts boundary from Content-Type header.
    pub fn extract_boundary(content_type: &String) -> Option<String> {
        let value: Vec<&str> = content_type.split(";").collect();

        if value.len() >= 2 {
            let content_type_text = value.get(1).unwrap().trim();
            let boundary = content_type_text.strip_prefix("boundary=").unwrap();
            return Some(boundary.to_string());
        }

        return None;
    }

    pub mod reader {
        use std::io::Read;
        use std::net::TcpStream;
        use crate::parser::multipart::{MultipartFormDataError, StreamReader};

        pub struct FormDataReader {
            pub stream: TcpStream,
            pub boundary_end_bytes: Vec<u8>,
            pub content_length: Option<usize>,
            // Size of bytes that has been already read
            pub bytes_read: usize,
            pub body_ended: bool,
            /// Store only some amount of bytes equals to the boundary end bytes
            body_buffer: Vec<u8>,
        }

        impl FormDataReader {
            pub fn new(stream: TcpStream, boundary: String, content_length: Option<usize>, body_read: usize) -> Self {
                let boundary_end = format!("--{}\r\n", boundary);
                let boundary_end_bytes = boundary_end.as_bytes().to_vec();
                let body_buffer = Vec::with_capacity(boundary_end_bytes.len());

                let body_ended;
                if let Some(content_length) = content_length {
                    body_ended = body_read >= content_length;
                } else if body_buffer.ends_with(&boundary_end_bytes) {
                    body_ended = true;
                } else {
                    body_ended = false;
                }

                return Self {
                    stream,
                    boundary_end_bytes,
                    content_length,
                    bytes_read: body_read,
                    body_ended,
                    body_buffer,
                };
            }

            /// Performs checks and updates status
            fn update_read_status(&mut self, new_chunk: &[u8]) {
                self.bytes_read += new_chunk.len();

                if self.content_length.is_some() {
                    let body_ended = self.bytes_read >= self.content_length.unwrap();
                    if body_ended {
                        self.body_ended = true;
                    }
                } else {
                    if self.body_buffer.ends_with(&self.boundary_end_bytes) {
                        self.body_ended = true;
                        return;
                    }

                    // Read is not finished yet, but we will prepare for next time
                    // If length of new chunk is more than the boundary end bytes, it means data is not ended yet.
                    // We can copy whole last bytes equivalent of boundary end bytes
                    if new_chunk.len() > self.boundary_end_bytes.len() {
                        self.body_buffer.clear();
                        let last_sice = &new_chunk[(self.boundary_end_bytes.len() - self.boundary_end_bytes.len())..self.boundary_end_bytes.len()];
                        self.body_buffer.extend(last_sice);
                    } else {
                        // If the chunk is smaller than the boundary length
                        // Merge old and new slice and save in the body_buffer
                        let start_index = self.boundary_end_bytes.len() - new_chunk.len() - 1;
                        let old_slice_to_copy = &self.body_buffer[start_index..].to_owned();

                        self.body_buffer.clear();
                        self.body_buffer.extend(old_slice_to_copy);
                        self.body_buffer.extend(new_chunk);
                    }
                }
            }
        }

        impl StreamReader for FormDataReader {
            fn get_chunk(&mut self) -> Result<Vec<u8>, MultipartFormDataError> {
                if self.body_ended {
                    return Err(MultipartFormDataError::BodyReadEnd);
                }

                const BUFFER_SIZE: usize = 8 * 1024; // 8 KiB
                let mut buffer = [0u8; BUFFER_SIZE];
                let result = self.stream.read(&mut buffer);

                if !result.is_ok() {
                    return Err(MultipartFormDataError::Others("Unable to read stream. May be client disconnected."));
                }

                let read_size = result.unwrap();
                if read_size == 0 {
                    return Err(MultipartFormDataError::Others("Bytes read size is 0. Probably client disconnected."));
                }

                let chunk_slice = &buffer[0..read_size];
                self.update_read_status(&chunk_slice);

                let chunk = Vec::from(chunk_slice);
                return Ok(chunk);
            }

            fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, MultipartFormDataError> {
                if self.body_ended {
                    return Err(MultipartFormDataError::BodyReadEnd);
                }

                let mut buffer: Vec<u8> = vec![0u8; size];
                let result = self.stream.read_exact(&mut buffer);
                if !result.is_ok() {
                    return Err(MultipartFormDataError::Others("Unable to read stream. May be client disconnected."));
                }

                self.update_read_status(&buffer);
                return Ok(buffer);
            }
        }
    }

    #[derive(Debug)]
    pub struct FormPart {
        pub name: Option<String>,
        pub filename: Option<String>,
        pub content_type: Option<String>,
        pub temp_file: Option<NamedTempFile>,
        pub value: Option<Vec<u8>>,
    }

    #[derive(Debug)]
    pub struct FormPartLimit {
        pub max_size: Option<usize>,
        pub content_type: Option<String>,
    }

    #[derive(Debug)]
    pub struct Limits {
        pub max_body_size: Option<usize>,
        pub max_header_size: Option<usize>,
        pub form_part_limits: HashMap<String, FormPartLimit>,
    }

    impl Limits {
        pub fn none() -> Self {
            return Self {
                max_body_size: None,
                max_header_size: None,
                form_part_limits: HashMap::new(),
            };
        }
    }

    #[derive(Debug)]
    pub enum FormPartResult {
        CheckNext,
        BodyCompleted,
    }

    impl FormPart {
        pub fn empty() -> Self {
            return FormPart {
                name: None,
                filename: None,
                content_type: None,
                temp_file: None,
                value: None,
            };
        }
    }

    /// It expects that the header has been completely read including \r\n\r\n characters.
    ///
    ///
    /// Here's example:
    /// ```markdown
    /// ----------------------------648887867674240986891965
    /// Content-Disposition: form-data; name="name"
    ///
    /// John Doe
    /// ----------------------------648887867674240986891965
    /// Content-Disposition: form-data; name="file"; filename="a.txt"
    /// Content-Type: text/plain
    ///
    ///
    /// hello
    ///
    /// ----------------------------648887867674240986891965
    /// Content-Disposition: form-data; name="gender"
    ///
    /// male
    /// ----------------------------648887867674240986891965--
    /// ```
    pub fn parse<T: StreamReader>(partial_bytes: Vec<u8>, headers: &Headers, reader: T, limits: Limits)
                                  -> Result<Vec<FormPart>, MultipartFormDataError> {
        let content_type_bytes = headers.get("Content-Type");

        let content_type: String;
        if let Some(content_type_bytes) = content_type_bytes {
            content_type = content_type_bytes.get(0).unwrap().to_owned();
        } else {
            return Err(MultipartFormDataError::InvalidMultiPart("Content-Type header missing."));
        };

        let multipart_boundary: String;
        if let Some(boundary) = extract_boundary(&content_type) {
            multipart_boundary = boundary;
        } else {
            return Err(MultipartFormDataError::InvalidMultiPart("Unable to extract multipart boundary."));
        }

        // Check if the client body is larger than the limit
        if let Some(max_body_size) = limits.max_body_size {
            if let Some(content_length) = headers::content_length(&headers) {
                if content_length > max_body_size {
                    return Err(MultipartFormDataError::MaxBodySizeExceed("Maximum specified body size exceed."));
                }
            }
        }

        let body_buffer = Vec::from(partial_bytes);
        return parse_body_parts(reader, body_buffer, &multipart_boundary, limits);
    }

    pub fn parse_body_parts<T: StreamReader>(mut reader: T, mut body_buffer: Vec<u8>, boundary: &String,
                                             limits: Limits) -> Result<Vec<FormPart>, MultipartFormDataError> {
        let mut form_parts = Vec::new();

        // Remove starting boundary first. It will make parsing easy by matching \r\n--{boundary}

        let start_boundary = format!("--{}\r\n", boundary);
        let start_boundary_bytes = start_boundary.as_bytes();

        // All the data is not be received. If not received try to read the required number bytes to make the boundary string.
        if body_buffer.len() <= start_boundary_bytes.len() {
            // Instead of reading bytes of some length, we will read exactly bytes required to prevent from reading again.
            let bytes_required = start_boundary_bytes.len() - body_buffer.len();
            let chunk_request_result = reader.get_exact(bytes_required);

            match chunk_request_result {
                Ok(chunk) => {
                    body_buffer.extend(chunk);
                }

                Err(error) => {
                    return Err(error);
                }
            }
        };

        if !body_buffer_starts_with_boundary(&body_buffer, start_boundary_bytes) {
            return Err(MultipartFormDataError::InvalidMultiPart("Body does not start with boundary"));
        }

        // Remove boundary header start
        body_buffer = Vec::from(&body_buffer[start_boundary_bytes.len()..]);

        // Now, we can start looping the form part contents.
        loop {
            // Extract header from form part
            let header_result = extract_form_part_header(
                &mut reader,
                &mut body_buffer,
                &limits,
            );
            if !header_result.is_ok() {
                return Err(header_result.unwrap_err());
            }

            let form_part_header = header_result.unwrap();
            let header_text = String::from_utf8_lossy(&form_part_header).to_string();

            // Parse header obtained above
            let header_parse_result = parse_form_part_header(header_text);
            if !header_parse_result.is_ok() {
                return Err(header_parse_result.unwrap_err());
            }

            // Obtain form part after parsing header.
            // This contains file metadata and form name, value
            let mut form_part = header_parse_result.unwrap();

            // Extract the body to value or temporary file.
            // If it is file, it will be available on form_part.temp_file else value
            let body_parse_result = extract_form_part_body(
                &mut reader,
                &mut body_buffer,
                boundary,
                &mut form_part,
                &limits,
            );

            match body_parse_result {
                Ok(result) => {
                    match result {
                        FormPartResult::BodyCompleted => {
                            form_parts.push(form_part);
                            return Ok(form_parts);
                        }

                        FormPartResult::CheckNext => {
                            form_parts.push(form_part);
                            // Continue looping
                        }
                    }
                }

                Err(error) => {
                    return Err(error);
                }
            }
        }
    }

    pub fn body_buffer_starts_with_boundary(body_buffer: &Vec<u8>, start_boundary_bytes: &[u8]) -> bool {
        // Check if the body buffer starts with start boundary or not. If not we will discard and don't process further.
        let extracted_boundary_slice = &body_buffer[0..start_boundary_bytes.len()];
        return extracted_boundary_slice == start_boundary_bytes;
    }

    /// Parses the form part header and removes the header from body buffer including \r\n\r\n bytes.
    ///
    /// ```markdown
    /// Content-Disposition: form-data; name="name"
    ///

    /// John Doe
    /// ----------------------------648887867674240986891965
    /// Content-Disposition: form-data; name="file"; filename="a.txt"
    /// Content-Type: text/plain
    ///
    ///
    /// ... continues
    /// ```
    pub fn extract_form_part_header<T: StreamReader>(reader: &mut T, body_buffer: &mut Vec<u8>, limits: &Limits)
                                                     -> Result<Vec<u8>, MultipartFormDataError> {
        // There can be one CRLF line break as well as two. Need to handle both cases.
        let header_end_bytes = b"\r\n\r\n";
        let mut form_part_header_buffer = Vec::new();

        let max_header_size = limits.max_header_size;

        loop {
            let scan_result = body_buffer.windows(header_end_bytes.len())
                .position(|window| window == header_end_bytes);

            if let Some(found_index) = scan_result {
                // Copy the found header to form part header
                form_part_header_buffer.extend(&body_buffer[0..found_index]);

                // If MAX_HEADER_SIZE exceeds, return error.
                if max_header_size.is_some() && (form_part_header_buffer.len() >= max_header_size.unwrap()) {
                    return Err(MultipartFormDataError::HeaderSizeExceed("Header size exceed max specified size"));
                }

                // Remove the found header including trailing header end bytes
                *body_buffer = Vec::from(&body_buffer[found_index + header_end_bytes.len()..]);
                return Ok(form_part_header_buffer);
            } else {
                // Header is not found yet. However, we copy the unmatched buffer too except last 4 bytes;
                // Last 4 bytes not copied to header buffer because it's half part may be available in the buffer next time
                // after new read. So we can't check if it ends or not.
                // If there is no enough data to copy to header buffer we ignore and fill more data to body buffer.
                let to_copy_to_header_buffer = body_buffer.len() as i32 - header_end_bytes.len() as i32;
                if to_copy_to_header_buffer > 0 {
                    // Append new data to header buffer
                    form_part_header_buffer.extend(header_end_bytes);
                    // Also remove copied data from body buffer
                    *body_buffer = Vec::from(&body_buffer[to_copy_to_header_buffer as usize..]);
                }

                // If MAX_HEADER_SIZE exceeds, return error.
                if max_header_size.is_some() && (form_part_header_buffer.len() >= max_header_size.unwrap()) {
                    return Err(MultipartFormDataError::HeaderSizeExceed("Header size exceed max specified size"));
                } else {
                    let request_new_chunk = reader.get_chunk();

                    match request_new_chunk {
                        Ok(new_chunk) => {
                            body_buffer.extend(new_chunk);
                        }

                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            };
        }
    }

    /// Expects only the header
    pub fn parse_form_part_header(part_header: String) -> Result<FormPart, MultipartFormDataError> {
        let mut form_part = FormPart::empty();

        let headers: Vec<&str> = part_header.split("\r\n").collect();

        // Splitting headers lines by \r\n
        for header_line in headers {
            // Parse individual header line and update the form part.
            parse_header_line(header_line, &mut form_part);
        }

        return Ok(form_part);
    }

    pub fn parse_header_line(line: &str, form_part: &mut FormPart) {
        let line = line.trim();

        if line.is_empty() {
            return;
        }

        let name_value: Vec<&str> = line.split(":").collect();
        if name_value.len() >= 2 {
            let header_name = name_value.get(0).unwrap().trim();
            let header_value = name_value.get(1).unwrap().trim();

            // If the header is Content-Disposition, extract the metadata
            if header_name.to_lowercase() == "Content-Disposition".to_lowercase() {
                parse_content_disposition_value(header_value, form_part);
            } else if header_name.to_lowercase() == "Content-Type".to_lowercase() {
                parse_content_type(header_value, form_part);
            }
        }
    }


    /// Expects value of Content-Disposition value.
    ///
    /// Example:
    /// ```markdown
    /// form-data; name="username"
    /// form-data; name="file"; filename="hello.txt"
    /// ```
    pub fn parse_content_disposition_value(value: &str, form_part: &mut FormPart) {
        let value = value.trim();

        if !value.starts_with("form-data;") {
            // Not a valid Content-Deposition value for form part header
            return;
        }

        let remaining = value.strip_prefix("form-data;").unwrap().trim();
        let pattern = Regex::new(r#"(?<attribute>\w+)="(?<value>[^"]*)""#).unwrap();

        for captured in pattern.captures_iter(remaining) {
            let attribute = &captured["attribute"];
            let value = &captured["value"];

            if attribute == "name" {
                form_part.name = Some(value.to_string());
            } else if attribute == "filename" {
                form_part.filename = Some(value.to_string());
            }
        }
    }

    pub fn parse_content_type(value: &str, form_part: &mut FormPart) {
        form_part.content_type = Some(value.to_string());
    }

    pub fn extract_form_part_body<T: StreamReader>(reader: &mut T, body_buffer: &mut Vec<u8>, boundary: &String,
                                                   form_part: &mut FormPart, limits: &Limits) ->
                                                   Result<FormPartResult, MultipartFormDataError> {
        let field_name = &form_part.name;

        let mut form_part_limit: Option<&FormPartLimit> = None;
        if field_name.is_some() {
            let field_name = field_name.clone().unwrap();
            form_part_limit = limits.form_part_limits.get(&field_name);
        }

        let is_file = form_part.filename.is_some();
        if is_file {
            return extract_form_file_body(reader, body_buffer, boundary, form_part, form_part_limit);
        }

        return extract_form_value(reader, body_buffer, boundary, form_part, form_part_limit);
    }

    /// It writes the file to temporary file.
    /// Example to copy file
    /// ```markdown
    /// // Example to copy temp file
    ///
    /// let filename = &form_part.filename.unwrap();
    /// let owned = filename.to_owned();
    /// let path = temp_file.path();
    ///
    /// fs::copy(path, owned).expect("Error copying");
    /// ```
    pub fn extract_form_file_body<T: StreamReader>(reader: &mut T, body_buffer: &mut Vec<u8>, boundary: &String,
                                                   form_part: &mut FormPart, form_part_limit: Option<&FormPartLimit>)
                                                   -> Result<FormPartResult, MultipartFormDataError> {
        // Create new tmp directory
        let temp_file_create = NamedTempFile::new();
        let mut temp_file;

        match temp_file_create {
            Ok(file) => {
                temp_file = file;
            }

            Err(_) => {
                return Err(MultipartFormDataError::Others("Error creating temporary file"));
            }
        }

        // Files can be ended with single CRLF line breaks as well as multiple.
        // \r\n and --\r\n are ignored to match later. These will decide whether there is next form part or body ends.
        let file_end_matcher = format!("\r\n--{}", boundary);
        let file_end_matching_bytes = file_end_matcher.as_bytes();

        let mut bytes_written: usize = 0;

        loop {
            let search_file_end = body_buffer.windows(file_end_matching_bytes.len())
                .position(|window| window == file_end_matching_bytes);

            // Position where file_end_matcher started matching
            if let Some(body_end_index) = search_file_end {
                // Check if file is empty or not. If body_end_index is 0, either file is empty or file is already
                // written but body end is just matched.

                if body_end_index > 0 {
                    // Body end position has been found
                    // Exact body bytes left range
                    let mut bytes_to_copy = &body_buffer[0..body_end_index];
                    bytes_written += bytes_to_copy.len();

                    // Check extra two bytes if the file body ends with \r\n. Some client adds double CRLF line breaks.
                    // If file body ends with \r\n, ignore
                    if bytes_to_copy.ends_with(b"\r\n") {
                        bytes_to_copy = &bytes_to_copy[0..bytes_to_copy.len() - 2];
                    }

                    let write_result = temp_file.write_all(bytes_to_copy);
                    if !write_result.is_ok() {
                        return Err(MultipartFormDataError::Others("Error writing to temporary file"));
                    }

                    // Remove copied data from body buffer including boundary by creating new array.
                    *body_buffer = Vec::from(&body_buffer[body_end_index + file_end_matching_bytes.len()..]);
                }

                // Check if the file size is more than the limit set.
                if form_part_limit.is_some() && (bytes_written > form_part_limit.unwrap().max_size.unwrap()) {
                    return Err(MultipartFormDataError::MaxFieldSizeExceed(
                        form_part.name.clone().unwrap().to_string(),
                        "The file is bigger than the maximum allowed size")
                    );
                }

                // Check if it is the last form content or still there are others.
                // If it is the last form part content, it will contain --\r\n in next bytes.
                // If it is not last the last form part content, there will be \r\n in next bytes.
                // Till now, we don't know if body is completed or not.

                let end_body_bytes = b"--\r\n";
                let next_part_bytes = b"\r\n";
                // Read exact 4 bytes if there is nothing in the body buffer else request required number of bytes.
                // 4 bytes should be there before completing request body.

                if body_buffer.len() < 4 {
                    // Amount of bytes to read
                    let bytes_to_read = 4 - body_buffer.len();

                    let request_new_chunk = reader.get_exact(bytes_to_read);
                    match request_new_chunk {
                        Ok(chunk) => {
                            body_buffer.extend(chunk);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }

                // Compare --\r\n
                let body_end_compare = &body_buffer[0..4];
                if body_end_compare == end_body_bytes {
                    // All form part has been parsed
                    body_buffer.clear();
                    if !temp_file.seek(SeekFrom::Start(0)).is_ok() {
                        return Err(MultipartFormDataError::Others("Error to seek start 0 temporary file."));
                    }

                    form_part.temp_file = Some(temp_file);
                    return Ok(FormPartResult::BodyCompleted);
                }

                // Compare \r\n
                let form_part_next_compare = &body_buffer[0..2];
                if form_part_next_compare == next_part_bytes {
                    // Remove \r\n bytes from the body buffer
                    *body_buffer = Vec::from(&body_buffer[2..]);

                    if !temp_file.seek(SeekFrom::Start(0)).is_ok() {
                        return Err(MultipartFormDataError::Others("Error seek to start 0 temporary file."));
                    }

                    form_part.temp_file = Some(temp_file);
                    return Ok(FormPartResult::CheckNext);
                }

                // None of the condition is satisfied. Problem with the request body.
                return Err(MultipartFormDataError::ParsingError("Form content did not end with \r\n"));
            } else {
                // Body end still not found. Add new chunk to body buffer
                // However we still write the data from the buffer except last bytes equal to the boundary match header.
                // We don't want to compare with half bytes of boundary which will never match.
                // Instead, keep last bytes of boundary bytes size still in the body buffer to compare later.

                // This many bytes can be copied to temp file if it's size > 0
                // Here 2 is the size of length of \r\n which can be ignorable from the file.
                // Some uses single CRLF line break as well as double line breaks.
                // Don't move data from buffer to file if the length of the buffer is smaller than the
                // ending boundary + \r\n bytes.

                let to_copy_size = body_buffer.len() as i32 - (file_end_matching_bytes.len() as i32 + 2);

                if to_copy_size > 0 {
                    let to_copy = &body_buffer[0..to_copy_size as usize];

                    let write_result = temp_file.write_all(to_copy);
                    if !write_result.is_ok() {
                        return Err(MultipartFormDataError::Others("Error writing to temporary file"));
                    }

                    // Remove copied bytes from the body buffer
                    *body_buffer = Vec::from(&body_buffer[to_copy_size as usize..]);
                    bytes_written += to_copy_size as usize;
                }

                if form_part_limit.is_some() && (bytes_written > form_part_limit.unwrap().max_size.unwrap()) {
                    return Err(MultipartFormDataError::MaxFieldSizeExceed(
                        form_part.name.clone().unwrap().to_string(),
                        "The file is bigger than the maximum allowed size"));
                }

                let request_new_chunk = reader.get_chunk();

                match request_new_chunk {
                    Ok(new_chunk) => {
                        body_buffer.extend(new_chunk);
                    }

                    Err(error) => {
                        return Err(error);
                    }
                }
            };
        };
    }

    pub fn extract_form_value<T: StreamReader>(reader: &mut T, body_buffer: &mut Vec<u8>, boundary: &String,
                                               form_part: &mut FormPart, form_part_limit: Option<&FormPartLimit>)
                                               -> Result<FormPartResult, MultipartFormDataError> {
        let value_end_matcher = format!("\r\n--{}", boundary);
        let value_end_matching_bytes = value_end_matcher.as_bytes();

        let mut value_buffer: Vec<u8> = Vec::new();
        let mut bytes_written: usize = 0;

        loop {
            let end_index = body_buffer.windows(value_end_matching_bytes.len())
                .position(|window| window == value_end_matching_bytes);

            if let Some(end_index) = end_index {
                // Either value is empty or value has already stored, but its end is just matched
                if end_index > 0 {
                    // Value end found
                    let mut to_copy_bytes = &body_buffer[..end_index];

                    if to_copy_bytes.ends_with(b"\r\n") {
                        to_copy_bytes = &to_copy_bytes[0..to_copy_bytes.len() - 2]
                    }

                    bytes_written += to_copy_bytes.len();
                    value_buffer.extend(to_copy_bytes);

                    // Remove partial value end boundary from body buffer
                    *body_buffer = Vec::from(&body_buffer[end_index + value_end_matching_bytes.len()..]);
                }

                // Check if the value bytes written is larger than the limit specified
                if form_part_limit.is_some() && (bytes_written > form_part_limit.unwrap().max_size.unwrap()) {
                    return Err(MultipartFormDataError::MaxFieldSizeExceed(
                        form_part.name.clone().unwrap().to_string(),
                        "The form field value size exceeds the limit specified",
                    ));
                }

                // Check if it is the last form content or still there are others.
                // If it is the last form part content, it will contain --\r\n in next bytes.
                // If it is not last the last form part content, there will be \r\n in next bytes.
                // Till now, we don't know if body is completed or not.

                let end_body_bytes = b"--\r\n";
                let next_part_bytes = b"\r\n";
                // Read exact 4 bytes if there is nothing in the body buffer else request required number of bytes.
                // 4 bytes should be there before completing request body.

                if body_buffer.len() < 4 {
                    // Amount of bytes to read
                    let bytes_to_read = 4 - body_buffer.len();

                    let request_new_chunk = reader.get_exact(bytes_to_read);
                    match request_new_chunk {
                        Ok(chunk) => {
                            body_buffer.extend(chunk);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }

                // Compare --\r\n
                let body_end_compare = &body_buffer[0..4];
                if body_end_compare == end_body_bytes {
                    // All form part has been parsed
                    body_buffer.clear();
                    form_part.value = Some(value_buffer);
                    return Ok(FormPartResult::BodyCompleted);
                }

                // Compare \r\n
                let form_part_next_compare = &body_buffer[0..2];
                if form_part_next_compare == next_part_bytes {
                    // Remove \r\n bytes from the body buffer
                    *body_buffer = Vec::from(&body_buffer[2..]);
                    form_part.value = Some(value_buffer);
                    return Ok(FormPartResult::CheckNext);
                }

                // None of the condition is satisfied. Problem with the request body.
                return Err(MultipartFormDataError::ParsingError("Form content did not end with \r\n"));
            } else {
                // Value end not found

                // Copy scanned values to buffer except last bytes equal to the length of value_end_matching_bytes
                // We left last some bytes equal to value_end_matching_bytes because we need to compare again.
                // Here 2 is the size of length of \r\n which can be ignorable from the value.
                // Some uses single CRLF line break as well as double line breaks.
                let to_copy_size = body_buffer.len() as i32 - (value_end_matching_bytes.len() as i32 + 2);
                if to_copy_size > 0 {
                    bytes_written += to_copy_size as usize;

                    // This many bytes can be copied to value_buffer
                    value_buffer.extend(&body_buffer[..to_copy_size as usize]);
                    // Remove copied bytes form body buffer
                    *body_buffer = Vec::from(&body_buffer[to_copy_size as usize..]);
                }

                if form_part_limit.is_some() && (bytes_written > form_part_limit.unwrap().max_size.unwrap()) {
                    return Err(MultipartFormDataError::MaxFieldSizeExceed(
                        form_part.name.clone().unwrap().to_string(),
                        "The form field value size exceeds the limit specified")
                    );
                }

                let request_new_chunk = reader.get_chunk();
                match request_new_chunk {
                    Ok(chunk) => {
                        body_buffer.extend(chunk);
                    }

                    Err(error) => {
                        return Err(error);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::io::{Read};
    use rand::{Rng};
    use crate::headers::Headers;
    use crate::parser::multipart::{StreamReader};
    use crate::parser::multipart::{
        extract_form_part_body,
        extract_form_value,
        FormPart,
        Limits,
        MultipartFormDataError,
        parse,
        parse_form_part_header,
    };

    struct ChunkReader {
        body_bytes: Vec<u8>,
        bytes_read: usize,
    }

    impl ChunkReader {
        fn new(body: &str, bytes_read: usize) -> Self {
            let body_bytes = body.as_bytes().to_vec();

            return ChunkReader {
                body_bytes,
                bytes_read,
            };
        }

        fn get_bytes_left(&self) -> usize {
            // Number of bytes that are left
            let bytes_left: i32 = self.body_bytes.len() as i32 - self.bytes_read as i32;

            // Simulate socket broken
            if bytes_left > 0 {
                return bytes_left as usize;
            }

            println!("Waiting forever...");
            println!("Socket connection broken.");
            return 0;
        }
    }

    impl StreamReader for ChunkReader {
        fn get_chunk(&mut self) -> Result<Vec<u8>, MultipartFormDataError> {
            // Number of bytes that are left
            let bytes_left = self.get_bytes_left();
            if bytes_left == 0 {
                return Err(MultipartFormDataError::BodyReadEnd);
            }

            let to_read = rand::thread_rng().gen_range(0..bytes_left + 1);
            let chunk = Vec::from(&self.body_bytes[self.bytes_read..&self.bytes_read + to_read]);
            self.bytes_read = self.bytes_read + to_read;
            return Ok(chunk);
        }

        fn get_exact(&mut self, size: usize) -> Result<Vec<u8>, MultipartFormDataError> {
            let bytes_left = self.get_bytes_left();
            if bytes_left == 0 {
                println!("Waiting...");
                println!("Body is already read");
                return Err(MultipartFormDataError::BodyReadEnd);
            }
            let chunk = &self.body_bytes[self.bytes_read..self.bytes_read + size];
            self.bytes_read = self.bytes_read + size;
            return Ok(Vec::from(chunk));
        }
    }

    const SAMPLE_BODY: &str = "----------------------------211628740782087473305609\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nJohn Doe\r\n----------------------------211628740782087473305609\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\nhello\n\r\n----------------------------211628740782087473305609\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\nContent-Type: text/plain\r\n\r\nhello\n\r\n----------------------------211628740782087473305609\r\nContent-Disposition: form-data; name=\"gender\"\r\n\r\nmale\r\n----------------------------211628740782087473305609--\r\n";
    const SAMPLE_BODY_2: &str = "--boundary123\r\nContent-Disposition: form-data; name=\"field1\"\r\n\r\nvalue1\r\n\r\n--boundary123\r\nContent-Disposition: form-data; name=\"file\"; filename=\"example.txt\"\r\nContent-Type: text/plain\r\n\r\nThis is the content of the file.\r\n--boundary123\r\nContent-Disposition: form-data; name=\"field2\"\r\n\r\nvalue2\r\n--boundary123--\r\n";

    #[test]
    fn test_parser() {
        let mut reader = ChunkReader::new(SAMPLE_BODY_2, 0);
        let request_chunk_result = reader.get_exact(SAMPLE_BODY_2.len());
        assert_eq!(true, request_chunk_result.is_ok());

        let mut headers: Headers = HashMap::new();
        // let content_type = vec!["multipart/form-data; boundary=--------------------------211628740782087473305609".to_string()];
        let content_type = vec!["multipart/form-data; boundary=boundary123".to_string()];
        headers.insert("Content-Type".to_string(), content_type);

        let partial_body = request_chunk_result.unwrap();
        let parse_result = parse(partial_body, &headers, reader, Limits::none());
        match parse_result {
            Ok(form_parts) => {
                println!("Parsing success:");

                for form_part in form_parts.iter() {
                    println!("Name: {}", form_part.name.as_ref().unwrap());
                    if form_part.value.as_ref().is_some() {
                        println!("Value: {:?}", String::from_utf8(form_part.value.as_ref().unwrap().to_vec()));
                    }
                }
            }

            Err(error) => {
                println!("Error: {:?}", error);
            }
        }
    }

    #[test]
    fn test_header_parser() {
        let header_sample_1 = "\r\nContent-Disposition: form-data; name=\"John Doe\"\r\n\r\n";
        let parsing_result = parse_form_part_header(header_sample_1.to_string());
        assert_eq!(true, parsing_result.is_ok());
        let form_part = parsing_result.unwrap();
        assert_eq!("John Doe", form_part.name.unwrap());

        let header_sample_2 = "Content-Disposition: form-data; name=\"file\"; filename=\"a.txt\"\r\n\
        Content-Type: text/plain\r\n\r\n";
        let parsing_result = parse_form_part_header(header_sample_2.to_string());
        assert_eq!(true, parsing_result.is_ok());
        let form_part = parsing_result.unwrap();

        assert_eq!(form_part.name.unwrap(), "file");
        assert_eq!(form_part.filename.unwrap(), "a.txt");
        assert_eq!(form_part.content_type.unwrap(), "text/plain");
    }

    #[test]
    fn test_extract_file_body() {
        let sample_body = "John Doe\r\n\r\n----------------------------163905767229441796406063\r\nContent-Disposition...";

        for _ in 0..10 {
            let mut form_part = FormPart {
                name: Some("file".to_string()),
                filename: Some("file.txt".to_string()),
                content_type: Some("text/html".to_string()),
                temp_file: None,
                value: None,
            };

            let mut reader = ChunkReader::new(sample_body, 0);
            let mut body_buffer = reader.get_chunk().unwrap();
            // let mut body_buffer = Vec::new();

            let boundary = "--------------------------163905767229441796406063".to_string();
            let result = extract_form_part_body(&mut reader, &mut body_buffer,
                                                &boundary, &mut form_part, &Limits::none());
            match result {
                Ok(res) => {
                    println!("{:?}", res);
                    let mut temp_file = &form_part.temp_file.unwrap();
                    // Example to copy temp file
                    // let filename = &form_part.filename.unwrap();
                    // let owned = filename.to_owned();
                    // let path = temp_file.path();
                    // std::fs::copy(path, owned).expect("Error copying");

                    let mut content = String::new();
                    temp_file.read_to_string(&mut content).expect("Error reading temporary file");
                    assert_eq!(content, "John Doe");
                }

                Err(_) => {
                    panic!("Multipart body parsing returned error.");
                }
            }
        }
    }

    #[test]
    fn test_extract_form_value() {
        let sample_body = "John Doe\r\n----------------------------163905767229441796406063\r\nContent-Disposition";
        let mut reader = ChunkReader::new(sample_body, 0);
        let mut body_buffer = reader.get_chunk().unwrap();
        let boundary = "--------------------------163905767229441796406063".to_string();
        let mut form_part = FormPart::empty();

        let result = extract_form_value(
            &mut reader,
            &mut body_buffer,
            &boundary,
            &mut form_part,
            None,
        );

        assert_eq!(true, result.is_ok());
        assert_eq!(b"John Doe", &form_part.value.unwrap().as_slice());
    }
}