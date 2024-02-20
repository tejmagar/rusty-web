pub mod status;
pub mod parser;
pub mod request;
pub mod headers;
pub mod response;

pub mod paths {
    use crate::request::Request;
    use crate::response::Response;

    pub type Paths = Vec<Path<fn(Request, Response)>>;
    pub type SinglePath = Path<fn(Request, Response)>;

    /// Path accepts pathname and view
    pub struct Path<T> {
        pub name: String,
        pub view: T,
    }

    impl<T> Path<T> {
        pub fn new(name: &str, view: T) -> Self {
            let name = name.to_string();

            return Self {
                name,
                view,
            };
        }
    }
}


pub mod server {
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::{Arc, RwLock};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread::spawn;
    use crate::headers::{parse_request_method_header, extract_headers};
    use crate::paths::{Paths, SinglePath};
    use crate::request::{Request};
    use crate::response::Response;

    /// Example usage
    /// ```rust
    /// use rusty_web::paths::{Path, Paths};
    /// use rusty_web::request::Request;
    /// use rusty_web::response::Response;
    /// use rusty_web::server::run_server;
    /// use rusty_web::status::Status;
    ///
    /// fn home(request: Request, mut response: Response) {
    ///    response.html(Status::Ok, "Home Page".to_string()).send();
    /// }
    ///
    /// fn main() {
    ///    let paths: Paths = vec![
    ///         Path::new("/", home),
    ///    ];
    ///
    ///    run_server("0.0.0.0:8080", paths);
    /// }
    /// ```
    pub fn run_server(listen_address: &str, paths: Paths) {
        println!("Running server in: http://{}", listen_address);
        let tcp = TcpListener::bind(listen_address);

        match tcp {
            Ok(listener) => {
                listen_connections(listener, paths);
            }

            Err(_) => {
                eprintln!("Failed to listen stream");
            }
        }
    }

    pub fn listen_connections(listener: TcpListener, paths: Paths) {
        let paths_lock = Arc::new(RwLock::new(paths));

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let paths = Arc::clone(&paths_lock);

                    spawn(move || {
                        serve_client(stream, paths);
                    });
                }

                Err(error) => {
                    print!("Error receiving stream: {}", error);
                }
            }
        }
    }

    pub struct Context {
        /// A same tcp stream can be used to serve multiple pages. Setting accept_next will continue
        /// to use same connection. Make sure to set `accept_next` to false if request
        /// body is not read completely. It is passed to both Request struct.
        pub accept_next: AtomicBool,
    }

    impl Context {
        pub fn dont_wait(&self) {
            self.accept_next.store(false, Ordering::Relaxed);
        }
    }

    fn serve_client(stream: TcpStream, paths: Arc<RwLock<Paths>>) {
        let context = Context {
            accept_next: AtomicBool::new(true),
        };

        let context_ref = Arc::new(context);

        while context_ref.accept_next.load(Ordering::Relaxed) {
            let stream = stream.try_clone().expect("Error cloning stream");
            decode_request(stream, paths.clone(), context_ref.clone());
        }
    }

    pub fn decode_request(mut stream: TcpStream, paths: Arc<RwLock<Paths>>,
                          context: Arc<Context>) {
        let mut header_start = String::new();
        let mut partial_body_bytes = Vec::new();

        const MAX_HEADER_SIZE: usize = 1024 * 1024; // 1 MiB
        let headers_result = extract_headers(
            &mut stream,
            &mut header_start,
            &mut partial_body_bytes,
            MAX_HEADER_SIZE,
        );

        if !headers_result.is_ok() {
            context.accept_next.store(false, Ordering::Relaxed);
            return;
        }

        let headers = headers_result.unwrap();

        let request_info = parse_request_method_header(&header_start.as_str());
        if !request_info.is_some() {
            context.accept_next.store(false, Ordering::Relaxed);
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }

        let (method, raw_path) = request_info.unwrap();

        // These states are shared among request and response
        let body_read = Arc::new(AtomicBool::from(false));
        let body_parsed = Arc::new(AtomicBool::from(false));

        let mut request = Request::new(context, stream, method, raw_path, headers,
                                       body_read.clone(), body_parsed.clone());
        request.setup();

        // Some bytes are read unintentionally from the body. Set read value in the struct.
        request.set_partial_body_bytes(partial_body_bytes);

        let mut matched_view: Option<&SinglePath> = None;

        let binding = paths.read().unwrap();
        for path in binding.iter() {
            if request.pathname == path.name {
                matched_view = Some(&path);
            }
        }

        if let Some(view) = matched_view {
            serve_page(request, view);
        } else {
            serve_not_found(request);
        }
    }

    fn serve_page(request: Request, matched_path: &SinglePath) {
        let response = Response::new(request.clone());
        (matched_path.view)(request, response);
    }

    fn serve_not_found(request: Request) {
        let mut response = Response::new(request);
        response.html(404, "404 NOT FOUND".to_string());
        response.send();
    }
}
