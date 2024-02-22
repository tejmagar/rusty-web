use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use tempfile::NamedTempFile;
use crate::headers;
use crate::headers::{Headers};
use crate::parser::{body, multipart, url_encoded};
use crate::parser::body::Limits;
use crate::parser::body::reader::BodyReader;
use crate::parser::multipart::{FormPart, MultipartFormDataError};
use crate::parser::multipart::reader::FormDataReader;
use crate::parser::url_encoded::{FormFields, UrlEncodedFormDataError};
use crate::parser::url_encoded::reader::UrlEncodedReader;
use crate::request::form::{FormFiles, FormData, FormFile};
use crate::server::Context;

fn map_first_vec_value(map: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
    if let Some(values) = map.get(key) {
        if values.len() > 0 {
            let value = values.get(0).unwrap();
            return Some(value.to_owned());
        }
    }

    return None;
}

pub mod form {
    use std::collections::HashMap;
    use tempfile::NamedTempFile;
    use crate::request::map_first_vec_value;

    pub struct FormFile {
        pub filename: String,
        pub temp_file: NamedTempFile,
    }

    pub type MapFirstString = HashMap<String, Vec<String>>;

    pub trait MapFirstStringMethod {
        fn value(&self, name: &str) -> Option<String>;
    }

    impl MapFirstStringMethod for MapFirstString {
        fn value(&self, name: &str) -> Option<String> {
            return map_first_vec_value(self, name);
        }
    }

    pub type FormData = HashMap<String, Vec<String>>;
    pub type FormFiles = HashMap<String, Vec<FormFile>>;

    pub trait FormFileMethods {
        fn value(&self, name: &str) -> Option<&FormFile>;
    }

    impl FormFileMethods for FormFiles {
        fn value(&self, name: &str) -> Option<&FormFile> {
            if let Some(values) = self.get(name) {
                if values.len() > 0 {
                    let value = values.get(0).unwrap();
                    return Some(value);
                }
            }

            return None;
        }
    }

    pub struct File {
        pub name: String,
        pub content_type: String,
    }
}

pub type QueryParams = HashMap<String, Vec<String>>;

pub struct Request {
    pub context: Arc<Context>,
    pub stream: TcpStream,
    pub method: String,
    pub raw_path: String,
    pub pathname: String,
    pub query_params: QueryParams,
    pub headers: Headers,
    pub partial_body: Option<Vec<u8>>,
    form_data: FormData,
    form_files: FormFiles,
    /// It specifies that body has been read already either some part or all.
    /// If body read is true, but body parse is false, need to change current connection to "keep-alive: close"
    /// It is because parsing body is probably failed.
    pub body_read: Arc<AtomicBool>,
    pub body_parsed: Arc<AtomicBool>,
}

impl Request {
    pub fn new(context: Arc<Context>, stream: TcpStream, request_method: String, raw_path: String,
               headers: HashMap<String, Vec<String>>, body_read: Arc<AtomicBool>,
               body_parsed: Arc<AtomicBool>) -> Self {
        let form_data = FormData::new();
        let form_files = FormFiles::new();

        let pathname = Self::pathname_from_raw(&raw_path);
        let query_params = headers::query_params_from_raw(&raw_path);

        return Request {
            context,
            stream,
            method: request_method,
            raw_path,
            pathname,
            query_params,
            headers,
            partial_body: None,
            form_data,
            form_files,
            body_read,
            body_parsed,
        };
    }

    fn pathname_from_raw(raw_path: &String) -> String {
        if let Some(index) = raw_path.find("?") {
            let slice = &raw_path.as_str()[0..index];
            return slice.to_string();
        }

        return raw_path.to_string();
    }

    pub fn setup(&mut self) {
        let content_length = headers::content_length(&self.headers);
        let request_method = self.method.to_uppercase();

        if matches!(request_method.as_str(), "GET" | "HEAD" | "OPTIONS" | "DELETE" | "TRACE" | "CONNECT") {
            // Same connections can be used fot these requests since there is no request body
            if !content_length.is_some() {
                // Content length is missing. Assuming there is no request body
                self.body_read.store(true, Ordering::Relaxed);
            }
        }
    }


    pub fn set_partial_body_bytes(&mut self, bytes: Vec<u8>) {
        self.partial_body = Some(bytes);
    }

    pub fn should_close_connection(&self) -> bool {
        let connection_type = headers::connection_type(&self.headers);

        if let Some(connection_type) = &connection_type {
            if connection_type.to_lowercase() == "keep-alive" && self.body_read.load(Ordering::Relaxed) {
                return false;
            }
        }

        return true;
    }

    pub fn body(&mut self) -> Option<NamedTempFile> {
        if self.body_read.load(Ordering::Relaxed) {
            eprintln!("Body already read");
            return None;
        }

        let content_length = headers::content_length(&self.headers);

        if !content_length.is_some() {
            eprintln!("Content-Length header is missing");
            return None;
        }

        let cloned_stream = self.stream.try_clone();
        if !cloned_stream.is_ok() {
            eprintln!("Failed to clone stream");
            return None;
        }

        let limits = Limits {
            max_body_size: 512 * 1024 * 1024, // 512 MiB
        };

        let mut partial_bytes: Vec<u8> = Vec::new();
        if let Some(partial) = self.partial_body.as_mut() {
            partial_bytes.extend(partial.clone());
            partial.clear();
        }

        let reader = BodyReader::new(cloned_stream.unwrap(), content_length.unwrap(),
                                     partial_bytes.len(), limits);

        let parse_result = body::parse(
            partial_bytes,
            &self.headers,
            reader,
        );

        self.body_read.store(true, Ordering::Relaxed);

        match parse_result {
            Ok(temp_file) => {
                self.body_parsed.store(true, Ordering::Relaxed);
                return Some(temp_file);
            }

            Err(error) => {
                eprintln!("Error: {:?}", error);
            }
        }

        return None;
    }

    pub fn form_data(&mut self) -> &mut FormData {
        if !self.body_read.load(Ordering::Relaxed) {
            self.parse_request_body();
        }
        return &mut self.form_data;
    }

    pub fn files(&mut self) -> &mut FormFiles {
        if !self.body_read.load(Ordering::Relaxed) {
            self.parse_request_body();
        }

        return &mut self.form_files;
    }

    pub fn parse_request_body(&mut self) {
        let content_type = headers::extract_content_type(&self.headers);

        // Silently return success empty form data if it does not have body
        if !content_type.is_some() {
            // Empty form data
            let content_length = headers::content_length(&self.headers);

            if content_length.is_some() && content_length.unwrap() > 0 {
                eprintln!("Body has content, but missing content type.");
            }

            return;
        }

        let content_type_binding = content_type.unwrap();
        let content_type_value = content_type_binding.trim();
        let content_length = headers::content_length(&self.headers);

        if content_type_value.starts_with("multipart/form-data;") {
            const MAX_BODY_SIZE: usize = 512 * 1024 * 1024; // 512 MiB
            const MAX_HEADER_SIZE: usize = 1024 * 1024; // 1 MiB
            const MAX_VALUE_SIZE: usize = 2 * 1024; // 1 MiB

            let limits = multipart::Limits {
                max_body_size: Some(MAX_BODY_SIZE),
                max_header_size: Some(MAX_HEADER_SIZE),
                max_value_size: Some(MAX_VALUE_SIZE),
                form_part_limits: HashMap::new(),
            };

            let result = self.multipart_form_data(
                content_type_value.to_string(),
                content_length,
                limits,
            );

            // Body read but yet don't know result.
            self.body_read.store(true, Ordering::Relaxed);

            match result {
                Ok(form_parts) => {
                    let (form_data, form_files) = self.multipart_form_data_and_files(form_parts);
                    // Set body parsed to true
                    self.body_parsed.store(true, Ordering::Relaxed);
                    self.form_data = form_data;
                    self.form_files = form_files;
                }

                Err(error) => {
                    self.body_parsed.store(true, Ordering::Relaxed);
                    eprintln!("Error: {:?}", error);
                }
            }
        } else if content_type_value.starts_with("application/x-www-form-urlencoded") {
            let limits = url_encoded::Limits {
                max_body_size: 2 * 1024 * 1024 // 2 MiB
            };

            if !content_length.is_some() {
                // Content-Length header is required for "application/x-www-form-urlencoded"
                eprintln!("Content-Length is missing.");
                return;
            }

            let result = self.parse_url_encoded(
                content_length.unwrap(),
                limits,
            );
            self.body_read.store(true, Ordering::Relaxed);

            match result {
                Ok(form_fields) => {
                    self.body_parsed.store(true, Ordering::Relaxed);
                    self.form_data = form_fields;
                }

                Err(error) => {
                    eprintln!("Error: {:?}", error);
                }
            }
        }
    }

    pub fn multipart_form_data(&mut self, content_type: String, content_length: Option<usize>,
                               limits: multipart::Limits) -> Result<Vec<FormPart>, MultipartFormDataError> {
        let boundary = multipart::extract_boundary(&content_type);
        if !boundary.is_some() {
            return Err(MultipartFormDataError::Others("Boundary is missing from Content-Type"));
        }

        // Copy partial body which was read unintentionally
        let partial_body;
        if let Some(partial) = self.partial_body.as_mut() {
            partial_body = partial.clone();
            partial.clear();
        } else {
            partial_body = Vec::new();
        }

        return match self.stream.try_clone() {
            Ok(cloned_stream) => {
                // This will work as source of data
                let reader = FormDataReader::new(
                    cloned_stream,
                    boundary.unwrap(),
                    content_length,
                    partial_body.len(),
                );

                multipart::parse(
                    partial_body,
                    &self.headers,
                    reader,
                    limits,
                )
            }
            Err(_) => {
                Err(MultipartFormDataError::Others("Failed to copy stream"))
            }
        };
    }

    pub fn parse_url_encoded(&mut self, content_length: usize, limits: url_encoded::Limits)
                             -> Result<FormFields, UrlEncodedFormDataError> {
        let mut partial_bytes = Vec::new();

        if let Some(partial_body) = self.partial_body.as_mut() {
            partial_bytes.extend(partial_body.clone());
            partial_body.clear();
        }

        let cloned_stream = self.stream.try_clone().expect("Failed to clone stream");
        let mut reader = UrlEncodedReader::new(
            cloned_stream,
            content_length,
            partial_bytes.len(),
        );

        return url_encoded::parse(partial_bytes, &self.headers, &mut reader, limits);
    }

    pub fn multipart_form_data_and_files(&self, form_parts: Vec<FormPart>) -> (FormData, FormFiles) {
        let mut form_data = FormData::new();
        let mut form_files = FormFiles::new();

        for form_part in form_parts {
            if !form_part.name.is_some() {
                continue;
            }


            if form_part.value.is_some() {
                // It is field value

                let name = form_part.name.unwrap();
                if !form_data.contains_key(&name) {
                    let vec = Vec::new();
                    form_data.insert(name.clone(), vec);
                }

                let values = form_data.get_mut(&name).unwrap();
                let value_bytes = form_part.value.expect("Error in value parsing");
                let value = String::from_utf8_lossy(value_bytes.as_slice());
                values.push(value.to_string());
            } else if form_part.filename.is_some() {
                // It is file type
                let name = form_part.name.unwrap();
                if !form_files.contains_key(&name) {
                    let vec = Vec::new();
                    form_files.insert(name.clone(), vec);
                }

                let values = form_files.get_mut(&name).unwrap();
                let temp_file = form_part.temp_file;

                let filename = form_part.filename.expect("Error in parsing file body. At least expected filename.");
                let temp_file = temp_file.expect("Error in parsing file body. At least expected one temp file.");
                let form_file = FormFile {
                    filename,
                    temp_file,
                };

                values.push(form_file);
            }
        }

        return (form_data, form_files);
    }
}

impl Clone for Request {
    fn clone(&self) -> Self {
        return Request {
            context: self.context.clone(),
            stream: self.stream.try_clone().unwrap(),
            method: self.method.clone(),
            raw_path: self.raw_path.clone(),
            pathname: self.pathname.clone(),
            query_params: self.query_params.clone(),
            headers: self.headers.clone(),
            partial_body: self.partial_body.clone(),
            // We are not copying value field and files
            form_data: FormData::new(),
            form_files: FormFiles::new(),
            body_read: self.body_read.clone(),
            body_parsed: self.body_parsed.clone(),
        };
    }
}


