## **Current Features Implemented**

### **1. Project Structure**

- **src/main.rs**: Main event loop, sets up servers on multiple ports, uses `mio` (epoll-based) for non-blocking I/O. Now integrates session management for static file requests.
- **src/serverConfig.rs**: Parses and holds server configuration (addresses, routes, error pages, etc.).
- **src/static_file.rs**: Serves static files from a directory, with basic error handling (NotFound, Forbidden).
- **src/upload_handler.rs**: Handles file uploads via POST, parses multipart/form-data, enforces size limits.
- **src/cgi.rs**: Runs a Python CGI script, passing request body and PATH_INFO.
- **src/requests.rs**: Contains HTTP request/response parsing and building logic (not yet fully integrated).
- **src/session_manager.rs**: Manages sessions and cookies, generates session IDs, and tracks session data.
- **src/config.json**: JSON config for servers, routes, error pages, etc.
- **src/html/**: Static HTML files for serving.
- **src/py.py**: Example Python CGI script.

### **2. Features Already Working**

- **Multiple server/port support**: Listens on multiple addresses/ports as defined in config.
- **Non-blocking, single-threaded event loop**: Uses `mio` (epoll) for all I/O.
- **Basic static file serving**: Serves HTML files for GET requests.
- **Basic config parsing**: Reads config.json for server setup, routes, and error pages.
- **CGI execution**: Can run a Python script as CGI, passing body and environment.
- **File upload handler**: Can parse multipart/form-data and save files (with size check).
- **Session/cookie management**: SessionManager creates and tracks sessions, sets/reuses session cookies for clients (currently for static file requests).

### **3. Features Partially or Not Yet Integrated**

- **Full HTTP/1.1 compliance**: Only basic GET parsing; POST/DELETE, headers, chunked encoding, cookies, sessions, etc. are not fully handled.
- **Routing logic**: Not yet using config to route requests to static, upload, or CGI handlers.
- **Error handling**: Error pages are defined in config, but not yet served for all error cases.
- **Timeouts**: No explicit request timeout handling.
- **Chunked/unchunked requests**: Not yet implemented.
- **Cookie/session management**: Implemented for static file requests; not yet integrated for uploads, CGI, or other handlers.
- **Directory listing, redirections, method restrictions**: Not implemented.
- **Integration of all modules**: The event loop only serves static files; upload and CGI are not yet called from the main loop.

---

## **Session Management Flow**

- On each request, the server extracts the `Cookie` header (if present).
- The `SessionManager` checks for a valid `session_id` in the cookies:
  - If found and valid, the session is reused.
  - If not found or invalid, a new session is created and a new `session_id` is generated.
- The server sets a `Set-Cookie` header in the response if a new session is created.
- Session data is stored in-memory and can be extended to track user-specific data.
- Currently, this flow is implemented for static file requests; other handlers will be integrated next.

---

## **Integration Plan (Step-by-Step)**

### **A. Centralize Request Handling**

- **Goal**: All requests (GET, POST, DELETE) are parsed and routed through a single function (e.g., `handle_request`).
- **How**: Use the parser in `requests.rs` to parse incoming requests. Route based on config and method.
- **Status**: Session/cookie logic is now present for static file requests.

### **B. Routing Logic**

- **Goal**: Use config to decide if a request should be served as static, upload, or CGI.
- **How**: For each request:
  - Match the path and method to a route in config.
  - If method not allowed, return 405.
  - If route has a CGI extension, call `run_cgi_script`.
  - If route is for upload, call `handle_file_upload`.
  - Otherwise, serve static file.

### **C. Error Handling**

- **Goal**: Serve custom error pages for 400, 403, 404, 405, 413, 500 as defined in config.
- **How**: On error, look up the error page path in config and serve it.

### **D. HTTP/1.1 Compliance**

- **Goal**: Support GET, POST, DELETE, headers, chunked encoding, cookies, sessions.
- **How**: Expand the parser and response builder in `requests.rs`. Add cookie/session logic to all handlers.
- **Status**: Session/cookie logic present for static file requests; expand to other handlers.

### **E. Timeout Handling**

- **Goal**: Disconnect clients if requests take too long.
- **How**: Use `mio`’s timer or track timestamps per connection.

### **F. Directory Listing, Redirections, Index Files**

- **Goal**: Support directory listing, redirections, and default index files as per config.
- **How**: Implement logic in the router to check for these settings.

### **G. Only One Epoll Call Per Communication**

- **Goal**: Ensure all reads/writes go through epoll, and only one call per client/server communication.
- **How**: Already mostly handled by `mio`, but review event loop for compliance.

---

## **What’s Missing (To-Do List)**

- [ ] Integrate `requests.rs` parser and response builder into the main event loop.
- [ ] Implement `handle_request` to route requests using config and call the correct handler (static, upload, CGI, error).
- [ ] Expand HTTP parsing to support all required methods, headers, chunked encoding, cookies, and sessions.
- [ ] Implement timeout logic for requests.
- [ ] Serve custom error pages for all error cases.
- [ ] Add directory listing, redirection, and index file support.
- [ ] Ensure all I/O is non-blocking and goes through epoll.
- [ ] Add tests for all features and error cases.

---

## **Integration Plan (Visual)**

```mermaid
flowchart TD
    A[Event Loop (mio)] --> B[Parse HTTP Request (requests.rs)]
    B --> C{Route by Config}
    C -- Static File --> D[static_file.rs]
    C -- Upload --> E[upload_handler.rs]
    C -- CGI --> F[cgi.rs]
    C -- Error --> G[Serve Error Page]
    D --> H[Build HTTP Response]
    E --> H
    F --> H
    G --> H
    H --> I[Write to Client (epoll)]
```

---

## **Summary Table**

| Feature                        | Status                  | File(s)                      |
|--------------------------------|------------------------|------------------------------|
| Multi-port server               | Working                | main.rs, serverConfig        |
| Non-blocking I/O (epoll)        | Working                | main.rs                      |
| Static file serving             | Working                | static_file.rs               |
| File upload (multipart)         | Implemented            | upload_handler.rs            |
| CGI (Python)                    | Implemented            | cgi.rs, py.py                |
| Config parsing                  | Working                | serverConfig.rs, config.json |
| Routing by config               | Not integrated         | main.rs, serverConfig        |
| Error pages                     | Not integrated         | main.rs, config.json         |
| HTTP/1.1 full compliance        | Partial                | requests.rs                  |
| Timeout handling                | Not implemented        | main.rs                      |
| Cookies/sessions                | Static files only      | main.rs, session_manager.rs  |
| Directory listing/redirection   | Not implemented        | (to do)                      |
| Chunked encoding                | Not implemented        | (to do)                      |

---

## **What to Do Next**

1. **Integrate the request parser and response builder** from `requests.rs` into the event loop in `main.rs`.
2. **Implement a central `handle_request` function** that uses config to route requests and calls the correct handler.
3. **Expand HTTP parsing and response logic** for all required features.
4. **Wire up error handling, timeouts, and session/cookie management.**
5. **Test each feature and error case.**

---

**If you want, I can provide:**

- A concrete code example for integrating the request parser and router.
- A checklist for each module’s integration.
- Guidance on any specific feature (e.g., cookies, chunked encoding, etc.).

Let me know how you’d like to proceed!
