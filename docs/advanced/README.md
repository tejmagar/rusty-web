# Advanced Usage

In Rusty Web, you have full control over the socket stream. You can stream the response
however you like.

## Extracting request body

To access raw request body, you can use `request.body()` method. For this `Content-Length` header must be specified in
the request. 

