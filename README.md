# Rusty Web

It is a lightweight web framework for rust developers.
It does not implement all the http RFC standards. However, you can implement your own by customizing it.
[Learn rusty web](https://tejmagar.github.io/rusty-web/)

## Installation

```
[dependencies]
rusty-web = "0.0.1"
```

## Sample

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


Above listed goals are not implemented, however you can write your own mechanism to handle above cases.

## Conclusion

This framework don't force you to follow particular format. You can stream response however you like.
