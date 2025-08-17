# Rust HTTP Server

A simple single-threaded HTTP/1.1 server written entirely in **Rust**.  
The implementation uses the [`mio`](https://crates.io/crates/mio) crate for non-blocking I/O (epoll/kqueue abstraction) and everything is contained in a single file for clarity.

---

## Features

- ✅ HTTP/1.1 compliant  
- ✅ Built with [`mio`](https://crates.io/crates/mio) (epoll/kqueue wrapper)  
- ✅ Single-process, single-threaded design  
- ✅ Handles multiple ports and multiple servers simultaneously  
- ✅ Request timeout handling  
- ✅ Supports `GET`, `POST`, and `DELETE` methods  
- ✅ File upload support with configurable body size limits  
- ✅ Cookie & session handling  
- ✅ Chunked & unchunked requests  
- ✅ CGI execution (one implemented CGI of choice)  
- ✅ Custom error pages (`400, 403, 404, 405, 413, 500`)  
- ✅ Optional directory listing  
- ✅ Configurable via a simple config file  

---

## Configuration

The server is driven by a configuration file.  
You can define:

- Host and one or more ports  
- Default server (when `server_name` doesn’t match)  
- Custom error pages  
- Client body size limit for uploads  
- Routes with:
  - Allowed methods  
  - Redirections  
  - Root directory / default file  
  - CGI handlers for file extensions  
  - Directory listing (on/off)  

### Example `config.conf`

```conf
server {
    server_address 127.0.0.1;
    port 8080;
    server_name myserver;

    error_page 404 /errors/404.html;
    client_max_body_size 10M;

    route / {
        root /var/www/html;
        index index.html;
        methods GET POST DELETE;
        autoindex off;
    }

    route /cgi-bin {
        root /var/www/cgi-bin;
        cgi .py /usr/bin/python3;
    }
}
