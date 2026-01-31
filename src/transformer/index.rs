use crate::analyze::Analysis;
use crate::assign::IdMapping;
use crate::chunking_adapter::{Chunk, ChunksResult};
use crate::graph::{EdgeType, GraphEdge, GraphNode, KnowledgeDAG, NodeType};
use crate::search;
use crate::similarity::{build_index_with_params, query_neighbors};
use crate::types::is_stopword;
use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDocument {
    pub id: String,
    pub title: String,
    pub path: String,
    pub category: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub word_count: usize,
    pub chunk_ids: Vec<String>,
    pub headings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_id: String,
    pub doc_id: String,
    pub doc_title: String,
    pub heading: Option<String>,
    pub chunk_type: String,
    pub token_count: usize,
    pub summary: String,
    pub previous_chunk_id: Option<String>,
    pub next_chunk_id: Option<String>,
    pub path: String,
    /// Related chunks with similarity scores (populated from knowledge DAG)
    pub related_chunks: Vec<RelatedChunk>,
    /// Hierarchical chunk level (summary/standard/detailed)
    pub chunk_level: String,
    /// Parent chunk ID (for hierarchical navigation)
    pub parent_chunk_id: Option<String>,
    /// Child chunk IDs (for hierarchical navigation)
    pub child_chunk_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedChunk {
    pub chunk_id: String,
    pub similarity: f32,
}

/// Intermediate result from document indexing phase
#[derive(Debug)]
struct DocumentIndexResult {
    documents: Vec<IndexDocument>,
    keywords: HashMap<String, Vec<String>>,
    document_tags: Vec<(String, Vec<String>, String)>,
}

#[allow(clippy::too_many_arguments)]
pub fn build_and_write_index(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    chunks_result: &ChunksResult,
    output_dir: &Path,
    project_name: &str,
    max_related_chunks: Option<usize>,
    hnsw_m: Option<usize>,
    hnsw_ef_construction: Option<usize>,
) -> Result<()> {
    // Phase 1: Build document index and extract metadata
    let doc_index = build_document_index(analyses, link_map, chunks_result)?;

    // Phase 2: Build knowledge graph
    let dag = build_knowledge_dag(
        &doc_index.documents,
        &chunks_result.chunks_metadata,
        &doc_index.document_tags,
        max_related_chunks,
        hnsw_m,
        hnsw_ef_construction,
    )?;

    // Phase 3: Build chunk metadata with related chunks from DAG
    let chunks_metadata = build_chunk_metadata(&chunks_result.chunks_metadata, &dag);

    // Phase 4: Compute graph analytics
    let analytics = compute_graph_analytics(&dag, &doc_index.documents);

    // Phase 5: Assemble and write index JSON
    let ctx = IndexAssemblyContext {
        documents: &doc_index.documents,
        chunks_metadata: &chunks_metadata,
        keywords: &doc_index.keywords,
        dag: &dag,
        analytics: &analytics,
        total_chunks: chunks_result.total_chunks,
        project_name,
    };
    let index_json = assemble_index_json(ctx)?;
    write_index_file(output_dir, &index_json)?;

    // Phase 6: Build Tantivy index (optional, best-effort)
    build_tantivy_index(output_dir, &doc_index.documents)?;

    Ok(())
}

/// Build document index from analyses and link mapping.
///
/// Extracts documents, keywords, and tags for downstream processing.
/// This is a pure data transformation with no I/O.
fn build_document_index(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    chunks_result: &ChunksResult,
) -> Result<DocumentIndexResult> {
    let mut documents = Vec::new();
    let mut keywords: HashMap<String, Vec<String>> = HashMap::new();
    let mut document_tags = Vec::new();

    for analysis in analyses {
        if let Some(mapping) = link_map.get(&analysis.source_path) {
            let tags = extract_tags(analysis);
            document_tags.push((mapping.id.clone(), tags.clone(), analysis.category.clone()));

            // Build keywords from headings
            for heading in &analysis.headings {
                for word in heading.text.split_whitespace() {
                    let word_lower = word.to_lowercase();
                    if word_lower.len() > 3 && !is_stopword(&word_lower) {
                        keywords
                            .entry(word_lower)
                            .or_default()
                            .push(mapping.id.clone());
                    }
                }
            }

            // Get chunk IDs for this document
            let chunk_ids: Vec<String> = chunks_result
                .chunks_metadata
                .iter()
                .filter(|c| c.doc_id == mapping.id)
                .map(|c| c.chunk_id.clone())
                .collect();

            documents.push(IndexDocument {
                id: mapping.id.clone(),
                title: analysis.title.clone(),
                path: format!("docs/{}", mapping.filename),
                category: analysis.category.clone(),
                tags,
                summary: analysis.first_paragraph.clone(),
                word_count: analysis.word_count,
                chunk_ids,
                headings: analysis.headings.iter().map(|h| h.text.clone()).collect(),
            });
        }
    }

    Ok(DocumentIndexResult {
        documents,
        keywords,
        document_tags,
    })
}

/// Build chunk metadata enriched with related chunks from the knowledge graph.
///
/// This is a pure data transformation - no I/O performed.
fn build_chunk_metadata(chunks: &[Chunk], dag: &KnowledgeDAG) -> Vec<ChunkMetadata> {
    chunks
        .iter()
        .map(|chunk| {
            // Get related chunks from the DAG
            let related = dag.get_related_chunks(&chunk.chunk_id);
            let related_chunks: Vec<RelatedChunk> = related
                .into_iter()
                .take(5) // Limit to top 5 related chunks
                .map(|(id, similarity)| RelatedChunk {
                    chunk_id: id,
                    similarity,
                })
                .collect();

            ChunkMetadata {
                chunk_id: chunk.chunk_id.clone(),
                doc_id: chunk.doc_id.clone(),
                doc_title: chunk.doc_title.clone(),
                heading: chunk.heading.clone(),
                chunk_type: chunk.chunk_type.clone(),
                token_count: chunk.token_count,
                summary: chunk.summary.clone(),
                previous_chunk_id: chunk.previous_chunk_id.clone(),
                next_chunk_id: chunk.next_chunk_id.clone(),
                path: format!(
                    "chunks/{}-{}.md",
                    chunk.chunk_id.replace(['/', '#'], "-"),
                    chunk.chunk_level.as_str()
                ),
                related_chunks,
                chunk_level: chunk.chunk_level.as_str().to_string(),
                parent_chunk_id: chunk.parent_chunk_id.clone(),
                child_chunk_ids: chunk.child_chunk_ids.clone(),
            }
        })
        .collect()
}

/// Graph analytics computed from the knowledge DAG.
#[derive(Debug)]
struct GraphAnalytics {
    topo_order: Vec<String>,
    reachability: HashMap<String, Vec<String>>,
    node_importance: HashMap<String, f32>,
}

/// Compute topological order, reachability, and node importance from the DAG.
///
/// This is a pure computation - no I/O performed.
fn compute_graph_analytics(dag: &KnowledgeDAG, documents: &[IndexDocument]) -> GraphAnalytics {
    let topo_order = dag.topological_order();

    let mut reachability: HashMap<String, Vec<String>> = HashMap::new();
    let mut node_importance: HashMap<String, f32> = HashMap::new();

    for doc in documents {
        let reachable = dag.reachable_from(&doc.id);
        let mut reachable_list: Vec<String> =
            reachable.into_iter().filter(|id| id != &doc.id).collect();
        reachable_list.sort();
        reachability.insert(doc.id.clone(), reachable_list);

        // Compute node importance (sum of outgoing edge weights)
        node_importance.insert(doc.id.clone(), dag.node_importance(&doc.id));
    }

    GraphAnalytics {
        topo_order,
        reachability,
        node_importance,
    }
}

/// Context for index JSON assembly - groups related parameters.
struct IndexAssemblyContext<'a> {
    documents: &'a [IndexDocument],
    chunks_metadata: &'a [ChunkMetadata],
    keywords: &'a HashMap<String, Vec<String>>,
    dag: &'a KnowledgeDAG,
    analytics: &'a GraphAnalytics,
    total_chunks: usize,
    project_name: &'a str,
}

/// Assemble the complete index JSON structure.
///
/// This is a pure data transformation - no I/O performed.
fn assemble_index_json(ctx: IndexAssemblyContext<'_>) -> Result<serde_json::Value> {
    let dag_stats = ctx.dag.statistics();
    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(json!({
        "version": "5.0",
        "project": ctx.project_name,
        "updated": timestamp,
        "stats": {
            "doc_count": ctx.documents.len(),
            "chunk_count": ctx.total_chunks,
            "avg_chunk_size_tokens": ctx.chunks_metadata.iter()
                .map(|c| c.token_count)
                .sum::<usize>()
                .checked_div(ctx.total_chunks)
                .unwrap_or(0),
            "graph": {
                "node_count": dag_stats.node_count,
                "edge_count": dag_stats.edge_count,
                "sequential_edges": dag_stats.sequential_edges,
                "related_edges": dag_stats.related_edges,
                "reference_edges": dag_stats.reference_edges
            }
        },
        "documents": ctx.documents,
        "chunks": ctx.chunks_metadata,
        "keywords": ctx.keywords,
        "graph": {
            "nodes": ctx.dag.nodes(),
            "edges": ctx.dag.edges(),
            "topological_order": ctx.analytics.topo_order,
            "reachability": ctx.analytics.reachability,
            "node_importance": ctx.analytics.node_importance,
            "statistics": dag_stats
        },
        "navigation": {
            "type": "contextual_retrieval_with_dag",
            "strategy": "50-100 token context prefix + H2 boundaries + knowledge DAG with semantic similarity",
            "avg_tokens_per_chunk": 170,
            "graph_enabled": true,
            "similarity_metric": "jaccard_on_tags_and_category",
            "min_similarity_threshold": 0.3
        }
    }))
}

/// Write the index JSON to disk.
fn write_index_file(output_dir: &Path, index: &serde_json::Value) -> Result<()> {
    let index_file = output_dir.join("INDEX.json");
    fs::write(index_file, serde_json::to_string_pretty(index)?)
        .map_err(|e| anyhow::anyhow!("Failed to write INDEX.json: {e}"))
}

/// Build Tantivy full-text search index.
///
/// This is a best-effort operation - failure only logs a warning
/// since search can fall back to INDEX.json.
fn build_tantivy_index(output_dir: &Path, documents: &[IndexDocument]) -> Result<()> {
    search::open_or_create_index(output_dir)
        .and_then(|index| search::index_documents(&index, documents.to_vec()))
        .map_err(|e| {
            eprintln!("Warning: Failed to build Tantivy index: {e}");
            eprintln!("Search will fall back to INDEX.json, but will be slower");
            anyhow::anyhow!("Tantivy index build failed (non-fatal): {e}")
        })
}

pub fn build_and_write_compass(
    analyses: &[Analysis],
    link_map: &HashMap<String, IdMapping>,
    output_dir: &Path,
) -> Result<()> {
    let mut by_category: HashMap<String, Vec<(String, String, Vec<String>)>> = HashMap::new();

    for analysis in analyses {
        if let Some(mapping) = link_map.get(&analysis.source_path) {
            let tags = extract_tags(analysis);
            by_category
                .entry(analysis.category.clone())
                .or_default()
                .push((analysis.title.clone(), mapping.filename.clone(), tags));
        }
    }

    let mut compass = format!(
        "---\nid: meta/navigation/compass\ntitle: Documentation Compass\ngenerated: {}\n---\n\n",
        chrono::Utc::now().to_rfc3339()
    );

    compass.push_str(&format!(
        "# Documentation Compass\n\n> **{} documents**\n\n",
        analyses.len()
    ));

    // By category
    for category in &["tutorial", "concept", "ref", "ops", "meta"] {
        if let Some(docs) = by_category.get(*category) {
            compass.push_str(&format!("## {}\n\n", category.to_uppercase()));
            for (title, filename, tags) in docs.iter().take(5) {
                let tag_str = tags
                    .iter()
                    .take(2)
                    .map(|t| format!("`{t}`"))
                    .collect::<Vec<_>>()
                    .join(" ");
                compass.push_str(&format!("- [{title}](./docs/{filename}) {tag_str}\n"));
            }
            compass.push('\n');
        }
    }

    let compass_file = output_dir.join("COMPASS.md");
    fs::write(compass_file, compass)?;

    Ok(())
}

/// Extract tags using functional composition
fn extract_tags(analysis: &Analysis) -> Vec<String> {
    std::iter::once(analysis.category.clone())
        .chain(
            analysis
                .headings
                .iter()
                .take(3)
                .flat_map(|h| h.text.split_whitespace())
                .filter(|word| word.len() > 4 && !is_stopword(&word.to_lowercase()))
                .map(|word| word.to_lowercase()),
        )
        .sorted()
        .dedup()
        .take(5)
        .collect()
}

/// Generate a simple embedding vector from tags and category.
/// Uses a bag-of-words approach with a fixed vocabulary built from all unique words.
/// Returns a sparse embedding where each dimension represents a word's presence.
fn generate_embedding_from_tags(
    tags: &[String],
    category: &str,
    vocabulary: &HashMap<String, usize>,
    embedding_dim: usize,
) -> Vec<f32> {
    let mut embedding = vec![0.0; embedding_dim];

    // Add tag contributions
    for tag in tags {
        if let Some(&idx) = vocabulary.get(tag) {
            if idx < embedding_dim {
                embedding[idx] = 1.0;
            }
        }
    }

    // Add category contribution (weighted higher)
    if let Some(&idx) = vocabulary.get(category) {
        if idx < embedding_dim {
            embedding[idx] = 2.0;
        }
    }

    // Normalize to unit vector for cosine similarity
    let magnitude: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        embedding.iter_mut().for_each(|x| *x /= magnitude);
    }

    embedding
}

/// Build vocabulary from all tags and categories
fn build_vocabulary(
    document_tags: &[(String, Vec<String>, String)],
) -> Result<HashMap<String, usize>> {
    let mut vocab = HashMap::new();
    let mut idx: usize = 0;

    for (_, tags, category) in document_tags {
        // Add category to vocabulary
        if !vocab.contains_key(category) && !category.is_empty() {
            vocab.insert(category.clone(), idx);
            idx = idx.checked_add(1).ok_or_else(|| {
                anyhow::anyhow!("Vocabulary index overflow - too many unique tags/categories")
            })?;
        }

        // Add tags to vocabulary
        for tag in tags {
            if !vocab.contains_key(tag) && !tag.is_empty() {
                vocab.insert(tag.clone(), idx);
                idx = idx.checked_add(1).ok_or_else(|| {
                    anyhow::anyhow!("Vocabulary index overflow - too many unique tags/categories")
                })?;
            }
        }
    }

    Ok(vocab)
}

/// Build a knowledge graph DAG from documents and chunks
#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn build_knowledge_dag(
    documents: &[IndexDocument],
    chunks: &[Chunk],
    document_tags: &[(String, Vec<String>, String)],
    max_related_chunks: Option<usize>,
    hnsw_m: Option<usize>,
    hnsw_ef_construction: Option<usize>,
) -> Result<KnowledgeDAG> {
    let mut dag = KnowledgeDAG::new();

    // Add document nodes
    for doc in documents {
        let node = GraphNode {
            id: doc.id.clone(),
            node_type: NodeType::Document,
            title: doc.title.clone(),
            category: Some(doc.category.clone()),
        };
        dag.add_node(node);
    }

    // Add chunk nodes
    for chunk in chunks {
        let node = GraphNode {
            id: chunk.chunk_id.clone(),
            node_type: NodeType::Chunk,
            title: format!(
                "{} - {}",
                chunk.doc_title,
                chunk.heading.as_ref().unwrap_or(&"Intro".to_string())
            ),
            category: None,
        };
        dag.add_node(node);
    }

    // Add parent-child edges (document -> chunks)
    for chunk in chunks {
        let edge = GraphEdge {
            from: chunk.doc_id.clone(),
            to: chunk.chunk_id.clone(),
            edge_type: EdgeType::Parent,
            weight: 1.0,
        };
        dag.add_edge(edge);
    }

    // Add sequential edges (previous -> next chunks)
    for chunk in chunks {
        if let Some(next_id) = &chunk.next_chunk_id {
            let edge = GraphEdge {
                from: chunk.chunk_id.clone(),
                to: next_id.clone(),
                edge_type: EdgeType::Sequential,
                weight: 1.0,
            };
            dag.add_edge(edge);
        }
    }

    // Detect and add related chunk edges using HNSW (O(n log n) instead of O(n²))
    let max_related = max_related_chunks.unwrap_or(5);
    const SIMILARITY_THRESHOLD: f32 = 0.3;

    if !chunks.is_empty() {
        // Build vocabulary from all tags and categories
        let vocabulary = build_vocabulary(document_tags)?;
        let embedding_dim = vocabulary.len().max(1); // At least 1 dimension

        // Generate embeddings for all chunks
        let embeddings: Vec<Vec<f32>> = chunks
            .iter()
            .map(|chunk| {
                let tags = document_tags
                    .iter()
                    .find(|(id, _, _)| id == &chunk.doc_id)
                    .map(|(_, tags, _)| tags.clone())
                    .unwrap_or_default();

                let category = document_tags
                    .iter()
                    .find(|(id, _, _)| id == &chunk.doc_id)
                    .map(|(_, _, cat)| cat.clone())
                    .unwrap_or_default();

                generate_embedding_from_tags(&tags, &category, &vocabulary, embedding_dim)
            })
            .collect();

        // Build HNSW index for O(log n) nearest neighbor search
        match build_index_with_params(&embeddings, hnsw_m, hnsw_ef_construction) {
            Ok(index) => {
                // Query top-k neighbors for each chunk
                for (i, chunk) in chunks.iter().enumerate() {
                    let chunk_tags = document_tags
                        .iter()
                        .find(|(id, _, _)| id == &chunk.doc_id)
                        .map(|(_, tags, _)| tags.clone())
                        .unwrap_or_default();

                    let chunk_category = document_tags
                        .iter()
                        .find(|(id, _, _)| id == &chunk.doc_id)
                        .map(|(_, _, cat)| cat.clone())
                        .unwrap_or_default();

                    let query_embedding = generate_embedding_from_tags(
                        &chunk_tags,
                        &chunk_category,
                        &vocabulary,
                        embedding_dim,
                    );

                    // Query HNSW for top-k neighbors (k+1 to account for self)
                    if let Ok(neighbors) =
                        query_neighbors(&index, &query_embedding, max_related.saturating_add(1))
                    {
                        let mut added_edges: usize = 0;
                        for (neighbor_idx, similarity) in neighbors {
                            // Skip self-edges and low-similarity matches
                            // Explicit bounds check to prevent panic on malformed HNSW indices
                            if neighbor_idx != i
                                && neighbor_idx < chunks.len()
                                && similarity >= SIMILARITY_THRESHOLD
                                && added_edges < max_related
                            {
                                let edge = GraphEdge {
                                    from: chunk.chunk_id.clone(),
                                    to: chunks[neighbor_idx].chunk_id.clone(),
                                    edge_type: EdgeType::Related,
                                    weight: similarity,
                                };
                                dag.add_edge(edge);
                                added_edges = added_edges.saturating_add(1);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // HNSW index build failed - skip related edges
                // This can happen with empty embeddings or invalid vectors
                eprintln!("Warning: HNSW index build failed ({e}), skipping related chunk edges");
                // Continue without adding related edges - document structure (parent/sequential) is preserved
            }
        }
    }

    Ok(dag)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use super::*;
    use crate::chunking_adapter::Chunk;
    use contextual_chunker::ChunkLevel;
    use std::collections::HashMap;

    /// Generate synthetic test chunks with realistic structure
    fn generate_test_chunks(n: usize) -> Vec<Chunk> {
        let docs_per_batch = (n as f64).sqrt().ceil() as usize;
        let chunks_per_doc = n.div_ceil(docs_per_batch);

        let mut chunks = Vec::with_capacity(n);

        for doc_idx in 0..docs_per_batch {
            let doc_id = format!("doc_{doc_idx:04}");
            let doc_title = format!("Document {doc_idx}");

            for chunk_idx in 0..chunks_per_doc {
                if chunks.len() >= n {
                    break;
                }

                let chunk_id = format!("chunk_{doc_idx}_{chunk_idx:04}");
                let previous_chunk_id = if chunk_idx > 0 {
                    Some(format!(
                        "chunk_{}_{:04}",
                        doc_idx,
                        chunk_idx.saturating_sub(1)
                    ))
                } else {
                    None
                };

                let next_chunk_id = if chunk_idx.saturating_add(1) < chunks_per_doc {
                    Some(format!(
                        "chunk_{}_{:04}",
                        doc_idx,
                        chunk_idx.saturating_add(1)
                    ))
                } else {
                    None
                };

                let chunk = Chunk {
                    chunk_id,
                    doc_id: doc_id.clone(),
                    doc_title: doc_title.clone(),
                    chunk_index: chunk_idx,
                    content: format!(
                        "Content for chunk {chunk_idx} in document {doc_idx}. This is sample documentation text."
                    ),
                    token_count: 256_usize.saturating_add(chunk_idx % 256),
                    heading: Some(format!("Section {chunk_idx}")),
                    chunk_type: "standard".to_string(),
                    previous_chunk_id,
                    next_chunk_id,
                    related_chunk_ids: Vec::new(),
                    summary: format!("Summary of chunk {chunk_idx} in doc {doc_idx}"),
                    chunk_level: ChunkLevel::Standard,
                    parent_chunk_id: None,
                    child_chunk_ids: Vec::new(),
                };

                chunks.push(chunk);
            }
        }

        chunks
    }

    /// Generate synthetic index documents
    fn generate_test_docs(chunks: &[Chunk]) -> Vec<IndexDocument> {
        let mut docs_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut docs_titles: HashMap<String, String> = HashMap::new();

        for chunk in chunks {
            docs_map
                .entry(chunk.doc_id.clone())
                .or_default()
                .push(chunk.chunk_id.clone());
            docs_titles
                .entry(chunk.doc_id.clone())
                .or_insert_with(|| chunk.doc_title.clone());
        }

        docs_map
            .into_iter()
            .enumerate()
            .map(|(idx, (doc_id, chunk_ids))| {
                let title = docs_titles
                    .get(&doc_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Document {idx}"));

                IndexDocument {
                    id: doc_id.clone(),
                    title,
                    path: format!("/docs/doc_{idx}.md"),
                    category: format!("Category {}", idx % 5),
                    tags: vec![
                        format!("tag_{}", idx % 3),
                        format!("tag_{}", idx.saturating_add(1) % 3),
                        format!("tag_{}", idx.saturating_add(2) % 3),
                    ],
                    summary: format!("Summary for document {idx}"),
                    word_count: 1000_usize.saturating_add(idx.saturating_mul(100)),
                    chunk_ids,
                    headings: vec![
                        "Introduction".to_string(),
                        "Content".to_string(),
                        "Conclusion".to_string(),
                    ],
                }
            })
            .collect()
    }

    /// Generate document tags for relationship detection
    fn generate_test_tags(chunks: &[Chunk]) -> Vec<(String, Vec<String>, String)> {
        let mut docs_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut docs_categories: HashMap<String, String> = HashMap::new();

        for chunk in chunks {
            docs_map
                .entry(chunk.doc_id.clone())
                .or_default()
                .push(chunk.chunk_id.clone());
            docs_categories
                .entry(chunk.doc_id.clone())
                .or_insert_with_key(|doc_id| {
                    let doc_num: usize = doc_id
                        .strip_prefix("doc_")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    format!("Category {}", doc_num % 5)
                });
        }

        docs_map
            .into_iter()
            .enumerate()
            .map(|(idx, (doc_id, _))| {
                let category = docs_categories
                    .get(&doc_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Category {}", idx % 5));

                let tags = vec![
                    format!("tag_{}", idx % 3),
                    format!("tag_{}", idx.saturating_add(1) % 3),
                    format!("tag_{}", idx.saturating_add(2) % 3),
                    "documentation".to_string(),
                    format!("section_{}", idx.saturating_div(10) % 10),
                ];

                (doc_id, tags, category)
            })
            .collect()
    }

    /// Test HNSW edge count linearity across multiple scales
    /// Verifies that edge count grows linearly (O(n)) not quadratically (O(n²))
    #[test]
    fn test_hnsw_edge_count_linear() {
        for n in [10, 100, 1000] {
            let chunks = generate_test_chunks(n);
            let docs = generate_test_docs(&chunks);
            let tags = generate_test_tags(&chunks);

            let dag = match build_knowledge_dag(&docs, &chunks, &tags, None, None, None) {
                Ok(d) => d,
                Err(e) => panic!("Failed to build knowledge DAG for edge count test: {e}"),
            };

            // N × max_related_chunks × safety_factor (1.5)
            // max_related_chunks = 20 in build_knowledge_dag
            let max_edges = n.saturating_mul(20).saturating_mul(15).saturating_div(10);

            assert!(
                dag.edges().len() < max_edges,
                "Edge count {} exceeds linear bound {} for {} chunks",
                dag.edges().len(),
                max_edges,
                n
            );
        }
    }

    /// Test that edge count is O(n log n), not O(n²)
    /// With HNSW, we expect at most max_related edges per node
    #[test]
    fn test_knowledge_dag_edge_count_is_linear() {
        const N: usize = 100;
        const MAX_RELATED: usize = 5;
        let max_related = MAX_RELATED;

        // Create test documents
        let documents: Vec<IndexDocument> = (0..10)
            .map(|i| IndexDocument {
                id: format!("doc_{i}"),
                title: format!("Document {i}"),
                path: format!("/path/doc_{i}.md"),
                category: format!("category_{}", i % 3),
                tags: vec![format!("tag_{}", i % 5), format!("tag_{}", (i + 1) % 5)],
                summary: format!("Summary for document {i}"),
                word_count: 100,
                chunk_ids: vec![],
                headings: vec!["Heading".to_string()],
            })
            .collect();

        // Create test chunks
        let chunks: Vec<Chunk> = (0..N)
            .map(|i| Chunk {
                chunk_id: format!("chunk_{i}"),
                doc_id: format!("doc_{}", i % 10),
                doc_title: format!("Document {}", i % 10),
                chunk_index: i,
                content: format!("Content for chunk {i}"),
                token_count: 100,
                heading: Some(format!("Heading {i}")),
                chunk_type: "standard".to_string(),
                previous_chunk_id: if i > 0 {
                    Some(format!("chunk_{}", i - 1))
                } else {
                    None
                },
                next_chunk_id: Some(format!("chunk_{}", i + 1)),
                related_chunk_ids: vec![],
                summary: format!("Summary {i}"),
                chunk_level: ChunkLevel::Standard,
                parent_chunk_id: None,
                child_chunk_ids: vec![],
            })
            .collect();

        // Create document tags
        let document_tags: Vec<(String, Vec<String>, String)> = (0..10)
            .map(|i| {
                (
                    format!("doc_{i}"),
                    vec![format!("tag_{}", i % 5), format!("tag_{}", (i + 1) % 5)],
                    format!("category_{}", i % 3),
                )
            })
            .collect();

        // Build the DAG
        let dag = match build_knowledge_dag(&documents, &chunks, &document_tags, None, None, None) {
            Ok(d) => d,
            Err(e) => panic!("Failed to build knowledge DAG for linear edge count test: {e}"),
        };

        // Get statistics
        let stats = dag.statistics();

        // Total edges include: parent edges (N), sequential edges (≈N), and related edges
        // Related edges should be at most N * max_related
        let max_expected_related_edges = N * max_related;

        // Count related edges
        let related_edges = dag.edges_by_type(&EdgeType::Related).len();

        println!("Total chunks: {N}");
        println!("Related edges: {related_edges}");
        println!("Max expected (N * {max_related}): {max_expected_related_edges}");
        println!("Total edges: {}", stats.edge_count);

        // Assert that related edges are bounded by O(n log n), not O(n²)
        // With HNSW and max_related=5, we expect at most N*5 related edges
        assert!(
            related_edges <= max_expected_related_edges,
            "Related edges {related_edges} exceeds O(n log n) bound {max_expected_related_edges}. This indicates O(n²) behavior!"
        );

        // For comparison: O(n²) would be 100*99/2 = 4950 edges
        let quadratic_edges = N * (N - 1) / 2;
        println!("Quadratic would be: {quadratic_edges} edges");
        // SAFETY: Edge counts in tests are small (< 10k), well within f64 precision (2^53)
        println!(
            "Ratio: {:.2}% of quadratic",
            (related_edges as f64 / quadratic_edges as f64) * 100.0
        );

        // Verify we're not in quadratic territory (should be < 20% of quadratic)
        assert!(
            related_edges < quadratic_edges / 5,
            "Edge count {} is too close to quadratic {} (should be < {})",
            related_edges,
            quadratic_edges,
            quadratic_edges / 5
        );
    }

    #[test]
    fn test_build_vocabulary() {
        let document_tags = vec![
            (
                "doc1".to_string(),
                vec!["rust".to_string(), "programming".to_string()],
                "tutorial".to_string(),
            ),
            (
                "doc2".to_string(),
                vec!["rust".to_string(), "web".to_string()],
                "guide".to_string(),
            ),
        ];

        let vocab = match build_vocabulary(&document_tags) {
            Ok(v) => v,
            Err(e) => panic!("Failed to build vocabulary from test document tags: {e}"),
        };

        // Should have 3 unique tags (rust, programming, web) + 2 categories (tutorial, guide) = 5 total
        // "rust" appears in both documents but is only counted once
        assert_eq!(vocab.len(), 5);
        assert!(vocab.contains_key("rust"));
        assert!(vocab.contains_key("programming"));
        assert!(vocab.contains_key("web"));
        assert!(vocab.contains_key("tutorial"));
        assert!(vocab.contains_key("guide"));
    }

    #[test]
    fn test_generate_embedding_from_tags() {
        let mut vocab = HashMap::new();
        vocab.insert("rust".to_string(), 0);
        vocab.insert("programming".to_string(), 1);
        vocab.insert("tutorial".to_string(), 2);

        let tags = vec!["rust".to_string(), "programming".to_string()];
        let category = "tutorial";

        let embedding = generate_embedding_from_tags(&tags, category, &vocab, 3);

        // Should be normalized
        let magnitude: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 0.01,
            "Embedding should be normalized to unit vector"
        );

        // All values should be non-negative since we only add positive weights
        assert!(embedding.iter().all(|&x| x >= 0.0));
    }

    #[test]
    fn test_empty_chunks_no_crash() {
        let documents = vec![];
        let chunks = vec![];
        let document_tags = vec![];

        let dag = match build_knowledge_dag(&documents, &chunks, &document_tags, None, None, None) {
            Ok(d) => d,
            Err(e) => panic!("Failed to build knowledge DAG with empty chunks: {e}"),
        };

        let stats = dag.statistics();
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }

    // ===========================================================================
    // Property-based tests for generate_embedding_from_tags
    // ===========================================================================
    // These tests verify invariants that should hold for all possible inputs

    /// Build a test vocabulary from a set of tags and categories
    fn build_test_vocabulary(
        all_tags: &[Vec<String>],
        all_categories: &[String],
    ) -> HashMap<String, usize> {
        let mut vocab = HashMap::new();
        let mut idx: usize = 0;

        for category in all_categories {
            if !vocab.contains_key(category.as_str()) && !category.is_empty() {
                vocab.insert(category.clone(), idx);
                idx = match idx.checked_add(1) {
                    Some(i) => i,
                    None => break,
                };
            }
        }

        for tags in all_tags {
            for tag in tags {
                if !vocab.contains_key(tag.as_str()) && !tag.is_empty() {
                    vocab.insert(tag.clone(), idx);
                    idx = match idx.checked_add(1) {
                        Some(i) => i,
                        None => break,
                    };
                }
            }
        }

        vocab
    }

    /// Property 1: Normalization - Result is unit vector (magnitude approx 1.0)
    /// For non-empty embeddings, the magnitude should always be 1.0
    #[test]
    fn proptest_embedding_normalization() {
        use proptest::prelude::*;

        let strategy = (
            prop::collection::vec(".*", 0..10),
            "[a-z]{1,20}",
            1..100usize,
        );

        proptest!(|(tags in strategy.0, category in strategy.1, embedding_dim in strategy.2)| {
            // Filter out empty strings and build vocabulary
            let clean_tags: Vec<String> = tags.into_iter()
                .filter(|s| !s.is_empty())
                .collect();

            let clean_category = if category.is_empty() { "default".to_string() } else { category };

            // Build vocabulary including all tags and category
            let vocab = build_test_vocabulary(&[clean_tags.clone()], &[clean_category.clone()]);

            // If vocabulary is empty, produce zero embedding (always has magnitude 0)
            if vocab.is_empty() {
                let dim = embedding_dim.max(1);
                let zero_embedding = generate_embedding_from_tags(&clean_tags, &clean_category, &vocab, dim);
                prop_assert_eq!(zero_embedding.len(), dim, "Zero embedding length mismatch");
            } else {
                // Ensure embedding_dim is at least 1
                let dim = embedding_dim.max(1);

                let embedding = generate_embedding_from_tags(&clean_tags, &clean_category, &vocab, dim);

                // Property: magnitude should be approximately 1.0 for non-zero embeddings
                let magnitude: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();

                // Allow small tolerance for floating point arithmetic
                prop_assert!(
                    magnitude > 0.0 && (magnitude - 1.0).abs() < 0.001,
                    "Embedding magnitude {} is not close to 1.0 (or is zero)",
                    magnitude
                );
            }
        });
    }

    /// Property 2: Length - Output dimension matches expected size
    #[test]
    fn proptest_embedding_length() {
        use proptest::prelude::*;

        let strategy = (
            prop::collection::vec("[a-z]{1,10}", 0..20),
            "[a-z]{1,10}",
            1..200usize,
        );

        proptest!(|(tags in strategy.0, category in strategy.1, embedding_dim in strategy.2)| {
            let vocab = build_test_vocabulary(&[tags.clone()], &[category.clone()]);

            let embedding = generate_embedding_from_tags(&tags, &category, &vocab, embedding_dim);

            // Property: output length must equal requested dimension
            prop_assert_eq!(
                embedding.len(),
                embedding_dim,
                "Embedding length {} != expected dimension {}",
                embedding.len(),
                embedding_dim
            );
        });
    }

    /// Property 3: Determinism - Same input produces same output
    #[test]
    fn proptest_embedding_determinism() {
        use proptest::prelude::*;

        let strategy = (
            prop::collection::vec("[a-z]{1,10}", 0..20),
            "[a-z]{1,10}",
            10..100usize,
        );

        proptest!(|(tags in strategy.0, category in strategy.1, embedding_dim in strategy.2)| {
            let vocab = build_test_vocabulary(&[tags.clone()], &[category.clone()]);

            // Generate embedding twice with same inputs
            let embedding1 = generate_embedding_from_tags(&tags, &category, &vocab, embedding_dim);
            let embedding2 = generate_embedding_from_tags(&tags, &category, &vocab, embedding_dim);

            // Clone for error message since prop_assert_eq! takes ownership
            let e1_repr = format!("{embedding1:?}");
            let e2_repr = format!("{embedding2:?}");

            // Property: outputs must be identical
            prop_assert_eq!(
                embedding1, embedding2,
                "Embeddings differ for same input: {} vs {}",
                e1_repr, e2_repr
            );
        });
    }

    /// Property 4: Empty input handling
    #[test]
    fn proptest_embedding_empty_input() {
        use proptest::prelude::*;

        // Test with various vocabulary sizes
        let vocab_strategy = prop::collection::hash_map("[a-z]{1,10}", 0..200usize, 0..50);

        proptest!(|(vocab in vocab_strategy, embedding_dim in 1..200usize)| {
            let empty_tags: Vec<String> = vec![];
            let empty_category = "";

            let embedding = generate_embedding_from_tags(&empty_tags, empty_category, &vocab, embedding_dim);

            // Property: empty input should produce zero vector (normalized to zero)
            // All elements should be 0.0
            prop_assert!(
                embedding.iter().all(|&x| x == 0.0),
                "Empty input should produce zero vector, got {:?}",
                embedding
            );

            // Property: length should still match
            prop_assert_eq!(
                embedding.len(),
                embedding_dim,
                "Zero vector length mismatch"
            );
        });
    }

    /// Property 5: Non-negative values
    /// All embedding values should be non-negative since we only add positive weights
    #[test]
    fn proptest_embedding_non_negative() {
        use proptest::prelude::*;

        let strategy = (
            prop::collection::vec("[a-z]{1,10}", 0..20),
            "[a-z]{1,10}",
            1..100usize,
        );

        proptest!(|(tags in strategy.0, category in strategy.1, embedding_dim in strategy.2)| {
            let vocab = build_test_vocabulary(&[tags.clone()], &[category.clone()]);

            let embedding = generate_embedding_from_tags(&tags, &category, &vocab, embedding_dim);

            // Property: all values must be >= 0
            prop_assert!(
                embedding.iter().all(|&x| x >= 0.0),
                "Found negative value in embedding: {:?}",
                embedding
            );
        });
    }

    /// Property 6: Order invariance
    /// Tags in different orders should produce the same embedding
    #[test]
    fn proptest_embedding_order_invariant() {
        use proptest::prelude::*;

        let tags_strategy = prop::collection::vec("[a-z]{1,10}", 2..10);
        let category_strategy = "[a-z]{1,10}";
        let dim_strategy = 10..100usize;

        proptest!(|(tags in tags_strategy, category in category_strategy, embedding_dim in dim_strategy)| {
            // Create a sorted version
            let mut tags_sorted = tags.clone();
            tags_sorted.sort();

            let vocab = build_test_vocabulary(&[tags.clone(), tags_sorted.clone()], &[category.clone()]);

            let embedding1 = generate_embedding_from_tags(&tags, &category, &vocab, embedding_dim);
            let embedding2 = generate_embedding_from_tags(&tags_sorted, &category, &vocab, embedding_dim);

            // Clone for error message since prop_assert_eq! takes ownership
            let e1_repr = format!("{embedding1:?}");
            let e2_repr = format!("{embedding2:?}");

            // Property: order should not matter
            prop_assert_eq!(
                &embedding1, &embedding2,
                "Embeddings differ for reordered tags: original={}, reordered={}",
                e1_repr, e2_repr
            );
        });
    }

    /// Property 7: Sparsity
    /// For large vocabularies, most dimensions should be zero
    #[test]
    fn proptest_embedding_sparsity() {
        use proptest::prelude::*;

        // Large vocabulary, few tags
        let tags_strategy = prop::collection::vec("[a-z]{1,10}", 1..5);
        let vocab_strategy = prop::collection::hash_map("[a-z]{1,10}", 0..200usize, 50..100);

        proptest!(|(tags in tags_strategy, category in "[a-z]{1,10}", vocab in vocab_strategy)| {
            let embedding_dim = vocab.len().max(1);

            let embedding = generate_embedding_from_tags(&tags, &category, &vocab, embedding_dim);

            // Count non-zero elements
            let non_zero_count = embedding.iter().filter(|&&x| x > 0.0).count();

            // Property: non-zero elements should not exceed unique tags + category
            let max_non_zero = tags.len().saturating_add(1); // +1 for category
            prop_assert!(
                non_zero_count <= max_non_zero,
                "Too many non-zero elements: {} > {} (tags: {})",
                non_zero_count, max_non_zero, tags.len()
            );
        });
    }
}
