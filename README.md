# Golink Service

A RESTful service written in Rust for managing golinks - short URL redirects with a pattern like `go/mylink`.

## Features

- **CRUD Operations**: Create, read, update, and delete golinks
- **Pattern Validation**: Enforces `go/[a-zA-Z0-9_-]+` format
- **Pagination Support**: Paginated results for large datasets
- **Dual Storage**: In-memory HashMap or SQLite database
- **RESTful API**: JSON-based HTTP endpoints
- **CORS Support**: Cross-origin resource sharing enabled
- **Error Handling**: Proper HTTP status codes and error responses

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/golinks` | Create a new golink |
| `GET` | `/golinks` | Get all golinks (supports pagination) |
| `GET` | `/golinks/{go/name}` | Get a specific golink |
| `PUT` | `/golinks/{go/name}` | Update a golink's URL |
| `DELETE` | `/golinks/{go/name}` | Delete a golink |

### Pagination Query Parameters

The `GET /golinks` endpoint supports optional pagination parameters:

| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| `page` | number | 1 | - | Page number (1-based) |
| `page_size` | number | 10 | 100 | Number of items per page |

## Usage

### Running the Service

```bash
cargo run
```

The service will start on `http://localhost:3030`.

#### Storage Backends

The service supports two storage backends:

**In-Memory (Default)**
```bash
cargo run
```

**SQLite Database**
```bash
# Set environment variables
export USE_SQLITE=1
export DATABASE_URL=golinks.db

# Run the service
cargo run
```

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

#### Get golinks with pagination
```bash
# Get first page with 5 items per page
curl -X GET "http://localhost:3030/golinks?page=1&page_size=5"

# Get second page with 10 items per page
curl -X GET "http://localhost:3030/golinks?page=2&page_size=10"

# Get first page with default page size (10)
curl -X GET "http://localhost:3030/golinks?page=1"
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

### Paginated Response
When using pagination parameters, the response structure changes to include pagination metadata:

```json
{
  "data": [
    {
      "id": "uuid-v4",
      "short_link": "go/example1",
      "url": "https://example1.com",
      "created_at": "2025-08-15T17:04:29.533013722+00:00"
    },
    {
      "id": "uuid-v4",
      "short_link": "go/example2", 
      "url": "https://example2.com",
      "created_at": "2025-08-15T17:03:15.421987654+00:00"
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 10,
    "total_items": 25,
    "total_pages": 3
  }
}
```

**Note**: When no pagination parameters are provided, the endpoint returns an array of golinks directly (maintaining backward compatibility).

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
- `sqlx`: SQLite database support (optional)
- `async-trait`: Async trait support

## License

This project is open source and available under the [MIT License](LICENSE).