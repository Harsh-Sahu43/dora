# DORA Node Package Specification

## 1. Introduction

The **DORA Node Package Specification** defines the packaging format used to distribute and execute nodes in the DORA ecosystem.

A **DORA node** represents a reusable computational unit within a robotic dataflow pipeline. Nodes may be implemented in multiple programming languages such as **Rust, Python, or C++**, and may be distributed through a centralized or self-hosted **DORA Node Registry**.

The package specification is designed to support three core systems:

### DORA CLI (Package Manager)

Responsible for dependency resolution, building, installing, and publishing node packages.

### DORA Runtime

Responsible for executing nodes and scheduling them on machines that satisfy their capability requirements.

### DORA Node Registry

Responsible for indexing, storing, and distributing node packages.

The package ecosystem is inspired by **Cargo + crates.io**, adapted for robotic node execution.

---

# 2. Package Components

A DORA node package consists of:

```
node-package/
 ├ Dora.toml
 ├ src/ or code files
 ├ resources/
 └ README.md
```

### Required file

```
Dora.toml
```

This file defines:

- package metadata
- node execution configuration
- dependency requirements
- runtime capabilities
- build instructions
- security verification

---

# 3. Manifest Format

The manifest file is written in **TOML**.

### Minimal manifest

```toml
[package]
name = "dora-example"
version = "0.1.0"

[node]
language = "python"
entrypoint = "main.py"
```

### Core sections

```
[package]
[node]
[dependencies]
[capabilities]
[build]
[security]
```

### Optional sections

```
[features]
[environment]
[resources]
[dev-dependencies]
```

---

# 4. Package Metadata

The `[package]` section defines metadata used by the **registry and CLI tools**.

### Example

```toml
[package]
name = "dora-yolo"
version = "0.1.0"
description = "YOLO object detection node"
license = "Apache-2.0"
authors = ["DORA Team"]
repository = "https://github.com/dora-rs/dora"
homepage = "https://dora.dev"
keywords = ["vision", "object-detection"]
```

### Required fields

| Field | Description |
|------|-------------|
| name | Unique package identifier |
| version | Semantic version |

### Optional fields

| Field | Description |
|------|-------------|
| description | Package description |
| authors | Package authors |
| license | License identifier |
| repository | Source repository |
| homepage | Project homepage |
| keywords | Tags for registry search |

---

# 5. Node Execution

The `[node]` section defines how the runtime executes the node.

### Example

```toml
[node]
language = "python"
entrypoint = "dora_yolo/main.py"
```

### Fields

| Field | Description |
|------|-------------|
| language | Implementation language |
| entrypoint | Path to executable or script |

### Supported languages (initial)

```
rust
python
cpp
```

### Future languages

```
wasm
container
```

### Execution examples

Python

```
python dora_yolo/main.py
```

Rust

```
./target/release/node_binary
```

C++

```
./bin/slam_node
```

---

# 6. Dependencies

Dependencies define other node packages required for execution.

### Example

```toml
[dependencies]
dora-camera = "1.0"
dora-parquet-recorder = "0.2"
```

Dependencies are resolved using **semantic version constraints**.

### Examples

```
"1.0"
">=0.2"
"^0.3"
```

Resolver builds a dependency graph:

```
dora-yolo
 ├ dora-camera
 └ dora-runtime
```

---

# 7. Capabilities

Capabilities describe hardware or system resources required by the node.

Used by the **runtime scheduler**.

### Example

```toml
[capabilities]
gpu = true
camera = true
cuda = ">=11.0"
```

### Example capabilities

| Capability | Description |
|------------|-------------|
| gpu | Requires GPU |
| camera | Requires camera |
| cuda | CUDA version requirement |
| network | Requires network access |

The runtime scheduler ensures nodes are deployed on machines satisfying these requirements.

---

# 8. Build Configuration

Defines how a node package is built.

### Example

```toml
[build]
type = "python"
python_version = ">=3.9"
```

### Rust node

```toml
[build]
type = "rust"
```

Equivalent command:

```
cargo build --release
```

### Python node

```toml
[build]
type = "python"
```

Equivalent command:

```
pip install -r requirements.txt
```

### C++ node

```toml
[build]
type = "cpp"
build_system = "cmake"
```

---

# 9. Environment Variables

The `[environment]` section defines environment variables required for execution.

### Example

```toml
[environment]
PYTHONPATH = "./lib"
CUDA_VISIBLE_DEVICES = "0"
```

---

# 10. Security

The `[security]` section defines package integrity verification.

### Example

```toml
[security]
checksum = "sha256:abc123..."
signature = "optional-signature"
```

### Fields

| Field | Description |
|------|-------------|
| checksum | SHA256 checksum |
| signature | Optional digital signature |

These fields are used by the **registry and package manager** to verify package integrity.

---

# 11. Lockfile Specification

The lockfile ensures **reproducible builds**.

### File name

```
Dora.lock
```

### Example

```toml
[[package]]
name = "dora-yolo"
version = "0.1.0"

dependencies = [
 "dora-camera 0.2.1",
 "dora-runtime 0.3.0"
]

[[package]]
name = "dora-camera"
version = "0.2.1"

[[package]]
name = "dora-runtime"
version = "0.3.0"
```

### Lockfile generation

```
Dora.toml
   ↓
Resolver
   ↓
Dependency Graph
   ↓
Dora.lock
```

Future builds read the lockfile instead of re-resolving dependencies.

---

# 12. Registry Architecture

The DORA registry consists of three components:

```
dora-node-registry
 ├ registry-server
 ├ registry-index
 └ packages
```

### Packages

Stores compressed package archives.

Example:

```
packages/
  dora-yolo/
     0.1.0.tar.gz
  dora-camera/
     0.2.1.tar.gz
```

---

# 13. Registry Index

The **index contains metadata only**, not packages.

Purpose:

- dependency resolution
- version lookup
- checksum verification

Directory structure follows a **hashed prefix strategy** similar to Cargo.

Example:

```
registry-index/
  do/
    ra/
      dora-camera
  do/
    ra/
      dora-yolo
```

---

# 14. Index Entry Format

Each file in the index contains JSON entries representing versions.

### Example

```json
{
  "name": "dora-camera",
  "vers": "0.2.1",
  "deps": [],
  "checksum": "abc123",
  "features": {}
}
```

### Example with dependencies

```json
{
  "name": "dora-yolo",
  "vers": "0.1.0",
  "deps": [
    {
      "name": "dora-camera",
      "req": "^0.2"
    }
  ],
  "checksum": "xyz987"
}
```

---

# 15. Dependency Resolver

The dependency resolver converts a manifest into a dependency graph.

### Workflow

```
read Dora.toml
     ↓
fetch registry index
     ↓
resolve versions
     ↓
build dependency graph
     ↓
generate Dora.lock
```

### Resolver components

```
resolver
 ├ manifest_loader
 ├ registry_client
 ├ version_solver
 ├ dependency_graph
 └ lockfile_writer
```

---

# 16. Manifest Parsing Architecture

The manifest is parsed through multiple layers.

```
Dora.toml
     ↓
TomlDoraManifest
     ↓
DoraManifest
     ↓
NodeSummary
     ↓
DependencyGraph
```

### Layer explanations

**TomlDoraManifest**

Raw TOML deserialization.

**DoraManifest**

Validated and normalized structure.

**NodeSummary**

Minimal structure used by the resolver.

---

# 17. CLI Commands

The manifest enables CLI workflows.

### Examples

```
dora init
dora validate
dora inspect
dora build
dora install
dora publish
```

### Example build flow

```
dora build
    ↓
parse Dora.toml
    ↓
resolve dependencies
    ↓
generate Dora.lock
    ↓
build node
```

---

# 18. Examples

### Python node

```toml
[package]
name = "dora-yolo"
version = "0.1.0"

[node]
language = "python"
entrypoint = "dora_yolo/main.py"

[dependencies]
dora-camera = "1.0"
```

### Rust node

```toml
[package]
name = "dora-lidar"
version = "0.2.0"

[node]
language = "rust"
entrypoint = "src/main.rs"

[dependencies]
dora-runtime = "0.1"
```

### C++ node

```toml
[package]
name = "dora-slam"
version = "0.3.0"

[node]
language = "cpp"
entrypoint = "bin/slam_node"
```

---

# 19. Design Principles

The DORA package specification follows several design principles.

### Language agnostic

Nodes may be implemented in any language.

### Minimal required fields

Only essential fields are mandatory.

### Extensible

New sections may be added without breaking existing packages.

### Runtime-aware

Manifest must provide sufficient information for scheduling and execution.

---

# 20. Ecosystem Workflow

The manifest forms the core of the DORA package ecosystem.

```
Dora.toml
    ↓
Manifest Parser
    ↓
Dependency Resolver
    ↓
Dora.lock
    ↓
Build System
    ↓
Runtime Execution
    ↓
Registry Publish
```
