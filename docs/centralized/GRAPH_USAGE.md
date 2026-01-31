# Graph Subcommand - Knowledge DAG Traversal

The `graph` subcommand allows you to explore the knowledge graph relationships between documents and chunks in your documentation.

## Overview

The knowledge graph is a Directed Acyclic Graph (DAG) that represents:
- **Documents** as nodes
- **Chunks** (semantic sections) as nodes
- **Relationships** as weighted edges between nodes

## Edge Types

- **Parent**: Document contains chunk (weight: 1.0)
- **Sequential**: Next chunk in document order (weight: 1.0)
- **Related**: Semantically similar content (weight: 0.3-1.0, based on Jaccard similarity)
- **References**: Explicit cross-references in documentation
- **ReferencedBy**: Backlinks from other documents

## Usage

### Basic Command

```bash
doc_transformer graph <NODE_ID> [OPTIONS]
```

### Options

- `<NODE_ID>`: The ID of the node to explore (required)
  - Document IDs: e.g., `"tutorial/general/getting-started"`
  - Chunk IDs: e.g., `"getting-started#0"`, `"doc-id#1"`
  
- `-i, --index-dir <DIR>`: Directory containing INDEX.json (default: current directory)

- `--reachable`: Show count of nodes reachable from this node (transitive closure)

### Examples

#### 1. Explore a Document Node

```bash
doc_transformer graph "tutorial/general/sample" --index-dir test_output
```

Output:
```
======================================================================
KNOWLEDGE GRAPH: tutorial/general/sample
======================================================================

Node: tutorial/general/sample (Document)
Title: Sample

Outgoing Edges: None

Incoming Edges: None

No relationships found

======================================================================
```

#### 2. Explore a Chunk Node

```bash
doc_transformer graph "sample#0" --index-dir test_output
```

Output:
```
======================================================================
KNOWLEDGE GRAPH: sample#0
======================================================================

Node: sample#0 (Chunk)
Title: Sample - Intro

Outgoing Edges (4):
  → sample#1 [Sequential, weight: 1.00]
     Sample - Installation
  → sample#1 [Related, weight: 1.00]
     Sample - Installation
  → sample#2 [Related, weight: 1.00]
     Sample - Basic Concepts
  → sample#3 [Related, weight: 1.00]
     Sample - Writing Your First Program

Incoming Edges (3):
  ← sample#1 [Related, weight: 1.00]
     Sample - Installation
  ← sample#2 [Related, weight: 1.00]
     Sample - Basic Concepts
  ← sample#3 [Related, weight: 1.00]
     Sample - Writing Your First Program

======================================================================
```

#### 3. Show Reachable Nodes

```bash
doc_transformer graph "sample#0" --index-dir test_output --reachable
```

Output includes:
```
Reachable: 12 nodes
```

This shows how many nodes can be reached by following edges from the starting node.

#### 4. Explore with Chunk ID containing #

```bash
doc_transformer graph "doc-id#0" --index-dir output
```

Chunk IDs with `#` characters are properly handled.

## Error Handling

### Node Not Found
```bash
$ doc_transformer graph "nonexistent" --index-dir test_output
Error: Node not found: nonexistent
```

### Missing INDEX.json
```bash
$ doc_transformer graph "sample#0" --index-dir /invalid
Error: INDEX.json not found at: /invalid/INDEX.json
Please run the transform command first.
```

### Missing Graph Data
If INDEX.json exists but doesn't contain graph data:
```
Error: INDEX.json missing graph data
```

## Output Format

### Node Information
- **Node ID**: The unique identifier
- **Node Type**: `Document` or `Chunk`
- **Title**: Truncated to 50 characters if longer (with `...`)

### Edge Information
Each edge displays:
- **Direction**: `→` for outgoing, `←` for incoming
- **Target/Source ID**: The other node in the relationship
- **Edge Type**: Parent, Sequential, Related, etc.
- **Weight**: Displayed with 2 decimal precision (e.g., `0.65`)
- **Title**: Title of the connected node (truncated to 40 chars)

### Special Cases
- **No Edges**: Displays "No relationships found"
- **Long Titles**: Automatically truncated with `...` suffix
- **Multiple Edge Types**: All edges between nodes are shown, even if multiple types exist

## Use Cases

### 1. Understanding Document Structure
Explore how a document breaks down into chunks:
```bash
doc_transformer graph "my-document"
```

### 2. Finding Related Content
See what other chunks are semantically similar:
```bash
doc_transformer graph "authentication#0"
```

### 3. Navigation Planning
Determine reachability for navigation features:
```bash
doc_transformer graph "index" --reachable
```

### 4. Debugging Relationships
Verify edge weights and types for quality assurance:
```bash
doc_transformer graph "troubleshooting#2"
```

## Implementation Details

### Graph Construction
The knowledge graph is built during the `transform` command:
1. Documents become nodes
2. Chunks become nodes
3. Parent-child edges link documents to their chunks
4. Sequential edges link chunks in reading order
5. Related edges connect semantically similar chunks (Jaccard similarity ≥ 0.3)

### Reachability Calculation
When `--reachable` is used:
- Performs depth-first search (DFS) from the starting node
- Counts all nodes reachable via outgoing edges
- Excludes the starting node from the count
- Uses transitive closure to find indirect relationships

### Performance
- Graph loaded from INDEX.json (one-time cost)
- Edge filtering performed in-memory
- Reachability uses efficient DFS with visited set
- Suitable for graphs with thousands of nodes

## Testing

The graph subcommand includes comprehensive tests:
- ✅ Finding node edges
- ✅ Node not found errors
- ✅ Graph command with valid nodes
- ✅ Chunk IDs with `#` characters
- ✅ Nodes with no edges
- ✅ Reachable nodes calculation
- ✅ Missing graph data handling
- ✅ Title truncation
- ✅ Edge weight precision
- ✅ Multiple edge types between nodes

Run tests:
```bash
cargo test graph
```

## See Also

- `transform` command - Builds the knowledge graph
- `search` command - Search documents and chunks by content
- INDEX.json schema - Graph data structure reference
