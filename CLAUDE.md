# CLAUDE.md - Dgraph Codebase Guide for AI Assistants

## Overview

Dgraph is a horizontally scalable, distributed GraphQL database with a graph backend. It provides ACID transactions, consistent replication, and linearizable reads. The project is written in Go and currently at version v25.

**Key characteristics:**
- **Two-tier architecture**: Zero (metadata/coordination) + Alpha (data/query processing)
- **Distributed consensus**: Uses Raft for replication across nodes
- **Query interfaces**: Native GraphQL, DQL (Dgraph Query Language), HTTP/gRPC APIs
- **Storage**: BadgerDB (embedded key-value store) with posting lists and MVCC

## Build Commands

```bash
# Build dgraph binary (outputs to ./dgraph/dgraph)
make dgraph

# Build and install to $GOPATH/bin
make install

# Build Docker image tagged as dgraph/dgraph:local
make image-local

# Show version and build info
make version

# Install Linux build dependencies
make linux-dependency
```

## Testing

Dgraph uses a custom test framework in the `t/` directory for Docker-based integration testing.

### Quick Start

```bash
cd t
make check            # Verify dependencies
go build .            # Build test runner

# Run tests
./t --pkg=dql                    # Test specific package
./t --pkg=graphql/e2e/normal     # Test GraphQL integration
./t --suite=core,vector          # Test suites
./t --test=TestParseCountValError # Run specific test function
./t -j=2                         # Run 2 parallel clusters
./t --skip-slow                  # Skip slow tests
./t --dry                        # List packages without running
./t --keep                       # Preserve containers for debugging
```

### Unit Tests

For packages that don't require Docker orchestration:

```bash
go test github.com/dgraph-io/dgraph/v25/dql
go test github.com/dgraph-io/dgraph/v25/posting
go test ./query/...
```

### Test Requirements

- **Docker**: Integration tests require Docker with sufficient memory. If tests hang, increase Docker memory allocation.
- **Go**: Version 1.24.3 or higher
- **gotestsum**: Required for test output collation
- **protoc**: Required for protocol buffer tests on Linux

## Code Formatting and Linting

```bash
# Format code
go fmt ./...

# Run full linting suite (recommended before commits)
trunk check

# Check specific files
trunk check <file>
```

**Linters enabled:** golangci-lint, trivy, actionlint, checkov, hadolint, markdownlint, shellcheck

## Directory Structure

### Core Components

| Directory | Purpose |
|-----------|---------|
| `dgraph/` | Main binary entry point and CLI commands |
| `dgraph/cmd/` | CLI subcommands (alpha, zero, bulk, live, cert, debug, etc.) |
| `query/` | Query execution engine |
| `posting/` | Posting list management (core data structure) with MVCC |
| `worker/` | Distributed worker coordination and Raft proposals |
| `schema/` | Schema management and validation |
| `dql/` | DQL parser (Dgraph Query Language) |
| `graphql/` | GraphQL support, admin API, subscriptions |
| `edgraph/` | Executive graph layer (high-level API, ACL, namespaces) |

### Supporting Components

| Directory | Purpose |
|-----------|---------|
| `protos/` | Protocol buffer definitions (`pb.proto`) |
| `types/` | Type system, conversions, geo filtering |
| `codec/` | Encoding/decoding |
| `conn/` | Connection management and pooling |
| `tok/` | Tokenization, full-text search, HNSW vector search |
| `x/` | Utility package (error handling, config, keys) |
| `algo/` | Algorithms (merge, intersect sorted lists) |
| `acl/` | Access Control Lists |
| `audit/` | Audit logging |
| `backup/` | Backup handling |
| `enc/` | Encryption at rest |
| `upgrade/` | Version upgrade handling |

### Testing Infrastructure

| Directory | Purpose |
|-----------|---------|
| `t/` | Custom test framework with Docker orchestration |
| `systest/` | System/integration tests |
| `testutil/` | Test utilities and helpers |
| `dgraphtest/` | Test client library |
| `dgraphapi/` | API helpers |

## Architecture

### Two-Tier System

1. **Zero nodes**: Metadata and coordination servers
   - Assigns UIDs and manages groups/shards
   - Maintains cluster membership state
   - Raft-based consensus for high availability

2. **Alpha nodes**: Data and query processing servers
   - Handles queries and mutations
   - Manages Raft state for data
   - Exposes HTTP (8080) and gRPC (9080) endpoints

### Default Test Cluster

Docker Compose configuration: 3 Zero nodes + 6 Alpha nodes
- Zero ports: 5080 (internal), 6080 (admin)
- Alpha ports: 8080 (HTTP), 9080 (gRPC)

## Key Code Patterns

### Error Handling

Use utilities from `x/error.go`:

```go
x.Check(err)                    // Fatal if err != nil
x.Checkf(err, "format: %v", v)  // Fatal with formatted message
x.CheckfNoTrace(err)            // Fatal without stack trace
x.AssertTrue(condition)         // Assert boolean condition
x.Ignore(err)                   // Deliberately ignore error
```

### Concurrency

- `sync.RWMutex` for read/write locks
- `x.SafeMutex` - Dgraph's wrapper for mutex safety
- `z.Closer` from ristretto for graceful shutdown
- Channel-based coordination patterns

### Context Usage

- `context.Context` for cancellation and tracing
- OpenTelemetry integration for distributed tracing
- gRPC metadata for inter-service communication

## Protocol Buffers

When modifying `.proto` files:

```bash
cd protos
make regenerate
```

Requires `protoc` 3.0.0+ and gogo protobuf plugin:
```bash
go get -u github.com/gogo/protobuf/protoc-gen-gofast
```

## Code Style Guidelines

- Follow [Go Code Review Comments](https://github.com/golang/go/wiki/CodeReviewComments)
- Wrap code and comments to 120 characters
- Use `go fmt` minimum, `trunk check` for full linting
- Avoid unnecessary vertical spaces

### License Header

Every new source file must begin with:

```go
/*
 * SPDX-FileCopyrightText: © 2017-2025 Istari Digital, Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
```

## Key Dependencies

| Dependency | Purpose |
|------------|---------|
| `dgraph-io/badger/v4` | Embedded key-value store |
| `dgraph-io/ristretto/v2` | Caching with jemalloc support |
| `etcd.io/etcd/raft/v3` | Raft consensus |
| `blevesearch/bleve/v2` | Full-text search |
| `dgraph-io/gqlparser/v2` | GraphQL parser |
| `google.golang.org/grpc` | RPC framework |
| `spf13/cobra` | CLI framework |
| `spf13/viper` | Configuration management |
| `prometheus/client_golang` | Metrics |

## CI/CD Workflows

Located in `.github/workflows/`:

- `ci-dgraph-core-tests.yml` - Unit tests
- `ci-dgraph-integration2-tests.yml` - Integration tests
- `ci-dgraph-systest-tests.yml` - System tests
- `ci-dgraph-vector-tests.yml` - Vector search tests
- `ci-dgraph-load-tests.yml` - Load testing
- `ci-dgraph-tests-arm64.yml` - ARM64 builds
- `codeql.yml` - Security scanning
- `trunk.yml` - Linting

## Common Development Tasks

### Adding a New CLI Command

1. Create directory under `dgraph/cmd/<command-name>/`
2. Implement command using Cobra framework
3. Register in `dgraph/cmd/root.go` subcommands slice

### Modifying Query Processing

- Query parsing: `dql/parser.go`
- Query execution: `query/query.go`
- Output formatting: `query/outputnode.go`

### Working with Posting Lists

Core data structure in `posting/`:
- `list.go` - In-memory representation with MVCC
- `index.go` - Indexing logic
- `mvcc.go` - Multi-version concurrency control

### Adding Tests

1. **Unit tests**: Add `*_test.go` files in same package
2. **Integration tests**: Add to `systest/` with appropriate `docker-compose.yml`
3. **GraphQL tests**: Add to `graphql/e2e/`

## Performance Considerations

- GOMAXPROCS set to 128 for high I/O throughput
- jemalloc enabled for memory management
- Memory monitoring in `dgraph/main.go`
- SIMD operations via `viterin/vek`

## Debugging

```bash
# Build with debug symbols
go build -gcflags="all=-N -l" ./dgraph

# Profile modes available via --profile_mode flag
dgraph alpha --profile_mode=cpu   # CPU profiling
dgraph alpha --profile_mode=mem   # Memory profiling
dgraph alpha --profile_mode=block # Block profiling
```

Use `dgraph debug` command for storage inspection utilities.

## Important Files

| File | Description |
|------|-------------|
| `dgraph/main.go` | Entry point with memory management |
| `dgraph/cmd/root.go` | Root CLI command and subcommand registration |
| `x/config.go` | Global configuration options |
| `x/error.go` | Error handling utilities |
| `x/keys.go` | Key generation and handling |
| `protos/pb.proto` | Main protocol buffer definitions |
| `dgraph/docker-compose.yml` | Default test cluster configuration |
