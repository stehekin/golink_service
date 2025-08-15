# Golink Service

A RESTful service written in Rust for managing golinks - short URL redirects with a pattern like `go/mylink`.

## Features

- **CRUD Operations**: Create, read, update, and delete golinks
- **Pattern Validation**: Enforces `go/[a-zA-Z_-]+` format
- **In-Memory Storage**: Thread-safe concurrent access
- **RESTful API**: JSON-based HTTP endpoints
- **CORS Support**: Cross-origin resource sharing enabled
- **Error Handling**: Proper HTTP status codes and error responses

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/golinks` | Create a new golink |
| `GET` | `/golinks` | Get all golinks |
| `GET` | `/golinks/{go/name}` | Get a specific golink |
| `PUT` | `/golinks/{go/name}` | Update a golink's URL |
| `DELETE` | `/golinks/{go/name}` | Delete a golink |

## Usage

### Running the Service

```bash
cargo run
```

The service will start on `http://localhost:3030`.

### API Examples

#### Create a golink
```bash
curl -X POST http://localhost:3030/golinks \
  -H "Content-Type: application/json" \
  -d '{"short_link": "go/github", "url": "https://github.com"}'
```

#### Get all golinks
```bash
curl -X GET http://localhost:3030/golinks
```

#### Get a specific golink
```bash
curl -X GET http://localhost:3030/golinks/go/github
```

#### Update a golink
```bash
curl -X PUT http://localhost:3030/golinks/go/github \
  -H "Content-Type: application/json" \
  -d '{"url": "https://github.com/explore"}'
```

#### Delete a golink
```bash
curl -X DELETE http://localhost:3030/golinks/go/github
```

## Data Structure

### Golink
```json
{
  "id": "uuid-v4",
  "short_link": "go/example",
  "url": "https://example.com",
  "created_at": "2025-08-15T17:04:29.533013722+00:00"
}
```

### Create Request
```json
{
  "short_link": "go/example",
  "url": "https://example.com"
}
```

### Update Request
```json
{
  "url": "https://new-example.com"
}
```

## Architecture

- **`main.rs`**: HTTP server setup and routing
- **`service.rs`**: Business logic, data models, and handlers

## Dependencies

- `tokio`: Async runtime
- `warp`: Web framework
- `serde`: Serialization/deserialization
- `uuid`: UUID generation
- `regex`: Pattern validation
- `chrono`: Timestamp handling

## License

This project is open source and available under the [MIT License](LICENSE).