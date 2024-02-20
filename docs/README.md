# Getting Started

Rusty Web is a lightweight web framework for rust developers.

## Installation

```
[dependencies]
rusty-web = "0.0.1"
```

## Basic Usage

```rust
use rusty_web::paths::{Path, Paths};
use rusty_web::request::Request;
use rusty_web::response::Response;
use rusty_web::server::run_server;
use rusty_web::status::Status;

fn home(request: Request, mut response: Response) {
    response.html(Status::Ok, "Home Page".to_string()).send();
}

fn about(request: Request, mut response: Response) {
    response.html(Status::Ok, "About Us".to_string()).send();
}

fn main() {
    let paths: Paths = vec![
        Path::new("/", home),
        Path::new("/about/", about),
    ];

    run_server("0.0.0.0:8080", paths);
}
```


