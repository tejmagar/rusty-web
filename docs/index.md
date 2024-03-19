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

## Handling file Upload

Handling file is much easier in Rusty Web.

```rust
fn upload(mut request: Request, mut response: Response) {
    if request.method == "POST" {
        let files = request.files();
        println!("FILES {:?}", files.keys());

        let file = files.value("file");
        println!("Some {:?}", file.is_some());
        response.html(Status::Ok, "Upload success".to_string()).send();
        return;
    }

    response.html(Status::BadRequest, "Upload failed".to_string()).send();
}
```

## Handling form data

Rusty Web supports `multipart/form-data` and `application/x-www-form-urlencoded` by default.
You can use to `request.form_data()` to access field values.

```rust
fn save_data(mut request: Request, mut response: Response) {
    if request.method == "POST" {
        let form_data = request.form_data();
        println!("Fields {:?}", form_data.keys());

        let username = form_data.value("username");
        println!("{:?}", username);

        response.html(Status::Ok, "Form success".to_string()).send();
        return;
    }

    response.html(Status::BadRequest, "Form failed".to_string()).send();
}
```

You can also use `request.form_data()` and `request.files()` together.

## Advanced Usage

In Rusty Web, you have full control over the socket stream. You can stream the response
however you like.

Go to [advanced usage tutorial](advanced/index)