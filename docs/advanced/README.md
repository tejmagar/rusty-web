# Advanced Usage

In Rusty Web, you have full control over the socket stream. You can stream the response
however you like.

## Request

You can access the common variables from request struct.

* request.query_params - It is a key values pair of query parameters. Type: `HashMap<String, Vec<String>>`.
* request.headers - It is a key values pair of request headers. `HashMap<String, Vec<String>>`
* request.stream - The socket TcpStream for sending/receiving data.
* request.context - This will contain the information about how to handle the further request.
* request.pathname - Current pathname of the request.
* request.raw_path - Full path of the request including query params.
* request.partial_body - This is a incomplete body bytes. Use this, if you are trying to implement custom response.

## Response

You can stream the HTTP response manually if you want.

* response.request - The request object is itself available in the response.

## Extracting request body

To access raw request body, you can use `request.body()` method. For this `Content-Length` header must be specified in
the request.
