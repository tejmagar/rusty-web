## Default Limits

When you call `request.files()`, `request.form_data()` or `request.body()`, Rusty Web automatically sets the maximum
allowed size.

- Request header: 1 MiB
- Multipart (multipart/form-data)
    - Overall maximum body: 512 MiB
    - Form Part header: 1 MiB
    - Form part file size: None
    - Form part value: 1 MiB
- application/x-www-form-urlencoded
    - Overall maximum body: 2 MiB
- Raw Body: 512 MiB

