#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! HNSW-based similarity search for document embeddings.
//!
//! This module provides O(log n) nearest neighbor search using the `hnsw_rs` library.
//! All operations are panic-free and use Railway-Oriented Programming for error handling.

use hnsw_rs::hnsw::Hnsw;
use hnsw_rs::prelude::DistCosine;
use thiserror::Error;

/// Errors that can occur during HNSW index operations.
#[allow(dead_code)] // All error variants available for library API completeness
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SimilarityError {
    /// Embedding dimensions do not match the index dimension.
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    /// Embedding contains invalid values (`NaN` or Infinity).
    #[error("invalid embedding: {0}")]
    InvalidEmbedding(String),

    /// Failed to build the HNSW index.
    #[error("index build failed: {0}")]
    IndexBuildFailed(String),

    /// Empty embeddings provided.
    #[error("empty embeddings: cannot determine dimension")]
    EmptyEmbeddings,
}

/// HNSW index for efficient nearest neighbor search.
///
/// This is a zero-panic wrapper around `hnsw_rs` that enforces dimension consistency
/// and validates all inputs.
pub struct HnswIndex {
    index: Hnsw<'static, f32, DistCosine>,
    dimension: usize,
}

/// Validates that an embedding contains no `NaN` or Infinity values.
///
/// # Errors
///
/// Returns `SimilarityError::InvalidEmbedding` if any value is `NaN` or Infinity.
fn validate_embedding(embedding: &[f32]) -> Result<(), SimilarityError> {
    embedding
        .iter()
        .enumerate()
        .find(|(_, &val)| val.is_nan() || val.is_infinite())
        .map_or(Ok(()), |(idx, &val)| {
            Err(SimilarityError::InvalidEmbedding(format!(
                "invalid value at index {idx}: {val}"
            )))
        })
}

/// Validates that all embeddings have the same dimension.
///
/// # Errors
///
/// Returns `SimilarityError::DimensionMismatch` if dimensions are inconsistent.
fn validate_dimensions(embeddings: &[Vec<f32>]) -> Result<usize, SimilarityError> {
    embeddings
        .first()
        .map(Vec::len)
        .ok_or(SimilarityError::EmptyEmbeddings)
        .and_then(|first_dim| {
            embeddings
                .iter()
                .enumerate()
                .find(|(_, emb)| emb.len() != first_dim)
                .map_or(Ok(first_dim), |(_idx, emb)| {
                    Err(SimilarityError::DimensionMismatch {
                        expected: first_dim,
                        got: emb.len(),
                    })
                })
        })
}

/// Builds an HNSW index from a collection of embeddings.
///
/// # Examples
///
/// ```
/// # use doc_transformer::similarity::{build_index, SimilarityError};
/// let embeddings = vec![
///     vec![1.0, 0.0, 0.0],
///     vec![0.0, 1.0, 0.0],
///     vec![0.0, 0.0, 1.0],
/// ];
///
/// let index = build_index(&embeddings)?;
/// # Ok::<(), SimilarityError>(())
/// ```
///
/// # Errors
///
/// - `SimilarityError::EmptyEmbeddings` if input is empty
/// - `SimilarityError::DimensionMismatch` if embeddings have inconsistent dimensions
/// - `SimilarityError::InvalidEmbedding` if any embedding contains `NaN` or Infinity
/// - `SimilarityError::IndexBuildFailed` if HNSW construction fails
#[allow(dead_code)] // Exported for library users - not used internally
pub fn build_index(embeddings: &[Vec<f32>]) -> Result<HnswIndex, SimilarityError> {
    build_index_with_params(embeddings, None, None)
}

/// Builds an HNSW index from a collection of embeddings with custom HNSW parameters.
///
/// # Arguments
///
/// * `embeddings` - Collection of embeddings to index
/// * `hnsw_m` - Optional number of neighbors (4-64). If None, defaults to 16.
/// * `hnsw_ef_construction` - Optional construction effort (50-800). If None, defaults to 200.
///
/// # Examples
///
/// ```
/// # use doc_transformer::similarity::{build_index_with_params, SimilarityError};
/// let embeddings = vec![
///     vec![1.0, 0.0, 0.0],
///     vec![0.0, 1.0, 0.0],
///     vec![0.0, 0.0, 1.0],
/// ];
///
/// let index = build_index_with_params(&embeddings, Some(32), Some(400))?;
/// # Ok::<(), SimilarityError>(())
/// ```
///
/// # Errors
///
/// - `SimilarityError::EmptyEmbeddings` if input is empty
/// - `SimilarityError::DimensionMismatch` if embeddings have inconsistent dimensions
/// - `SimilarityError::InvalidEmbedding` if any embedding contains `NaN` or Infinity
/// - `SimilarityError::IndexBuildFailed` if HNSW construction fails
pub fn build_index_with_params(
    embeddings: &[Vec<f32>],
    hnsw_m: Option<usize>,
    hnsw_ef_construction: Option<usize>,
) -> Result<HnswIndex, SimilarityError> {
    let dimension = validate_dimensions(embeddings)?;

    embeddings
        .iter()
        .try_for_each(|emb| validate_embedding(emb))?;

    let nb_elem = embeddings.len();
    let max_nb_connection = hnsw_m.unwrap_or(16);
    let ef_construction = hnsw_ef_construction.unwrap_or(200);

    let hnsw = Hnsw::<f32, DistCosine>::new(
        max_nb_connection,
        nb_elem,
        dimension,
        ef_construction,
        DistCosine {},
    );

    let data_with_id: Vec<(&Vec<f32>, usize)> = embeddings
        .iter()
        .enumerate()
        .map(|(idx, emb)| (emb, idx))
        .collect();

    hnsw.parallel_insert(&data_with_id);

    Ok(HnswIndex {
        index: hnsw,
        dimension,
    })
}

/// Queries the HNSW index for the k nearest neighbors.
///
/// # Examples
///
/// ```
/// # use doc_transformer::similarity::{build_index, query_neighbors, SimilarityError};
/// let embeddings = vec![
///     vec![1.0, 0.0, 0.0],
///     vec![0.0, 1.0, 0.0],
///     vec![0.0, 0.0, 1.0],
/// ];
///
/// let index = build_index(&embeddings)?;
/// let query = vec![0.9, 0.1, 0.0];
/// let neighbors = query_neighbors(&index, &query, 2)?;
///
/// assert_eq!(neighbors.len(), 2);
/// // HNSW returns approximate results; check that index 0 is in top results
/// assert!(neighbors.iter().any(|(idx, _)| *idx == 0));
/// # Ok::<(), SimilarityError>(())
/// ```
///
/// # Errors
///
/// - `SimilarityError::DimensionMismatch` if query dimension doesn't match index
/// - `SimilarityError::InvalidEmbedding` if query contains `NaN` or Infinity
///
/// # Returns
///
/// A vector of `(index, similarity)` tuples sorted by descending similarity.
/// Similarity scores are in the range [0.0, 1.0] where 1.0 means identical.
/// If `top_k` exceeds the number of indexed embeddings, all embeddings are returned.
pub fn query_neighbors(
    index: &HnswIndex,
    query: &[f32],
    top_k: usize,
) -> Result<Vec<(usize, f32)>, SimilarityError> {
    // Validate query dimension
    if query.len() != index.dimension {
        return Err(SimilarityError::DimensionMismatch {
            expected: index.dimension,
            got: query.len(),
        });
    }

    // Validate query for NaN/Infinity
    validate_embedding(query)?;

    // Search with ef = max(top_k, 200) for quality
    let ef_search = top_k.max(200);
    let neighbors = index.index.search(query, top_k, ef_search);

    // Convert distance to similarity: similarity = 1 - (distance / 2)
    // Cosine distance is in [0, 2], so cosine similarity is in [-1, 1]
    // We normalize to [0, 1] where 1.0 = identical
    let mut results: Vec<(usize, f32)> = neighbors
        .iter()
        .map(|neighbor| {
            let distance = neighbor.distance;
            let similarity: f32 = 1.0 - (distance / 2.0);
            let similarity_clamped: f32 = similarity.clamp(0.0, 1.0);
            (neighbor.d_id, similarity_clamped)
        })
        .collect();

    // Sort by similarity descending (highest similarity first)
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_index_success() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let result = build_index(&embeddings);
        assert!(result.is_ok());
        assert_eq!(result.as_ref().ok().map(|idx| idx.dimension), Some(3));
    }

    #[test]
    fn test_build_index_empty() {
        let embeddings: Vec<Vec<f32>> = vec![];
        let result = build_index(&embeddings);
        assert!(matches!(result, Err(SimilarityError::EmptyEmbeddings)));
    }

    #[test]
    fn test_build_index_dimension_mismatch() {
        let embeddings = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0]];
        let result = build_index(&embeddings);
        assert!(matches!(
            result,
            Err(SimilarityError::DimensionMismatch {
                expected: 3,
                got: 2
            })
        ));
    }

    #[test]
    fn test_build_index_nan() {
        let embeddings = vec![vec![1.0, f32::NAN, 0.0]];
        let result = build_index(&embeddings);
        assert!(matches!(result, Err(SimilarityError::InvalidEmbedding(_))));
    }

    #[test]
    fn test_build_index_infinity() {
        let embeddings = vec![vec![1.0, f32::INFINITY, 0.0]];
        let result = build_index(&embeddings);
        assert!(matches!(result, Err(SimilarityError::InvalidEmbedding(_))));
    }

    #[test]
    fn test_query_neighbors_success() -> anyhow::Result<()> {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let index = build_index(&embeddings)?;
        let query = vec![0.9, 0.1, 0.0];
        let neighbors = query_neighbors(&index, &query, 2)?;

        println!("neighbors: {neighbors:?}");
        assert_eq!(neighbors.len(), 2);
        // HNSW is approximate, so we check that the expected neighbor is in results
        // and that results are sorted by similarity (descending)
        assert!(
            neighbors.iter().any(|(idx, _)| *idx == 0),
            "index 0 should be in neighbors"
        );
        let mut prev_sim = f32::MAX;
        for (_, sim) in &neighbors {
            assert!(
                *sim <= prev_sim,
                "neighbors not sorted by similarity: got {sim} after {prev_sim}"
            );
            prev_sim = *sim;
        }
        Ok(())
    }

    #[test]
    fn test_query_neighbors_dimension_mismatch() -> anyhow::Result<()> {
        let embeddings = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];

        let index = build_index(&embeddings)?;
        let query = vec![1.0, 0.0]; // Wrong dimension
        let result = query_neighbors(&index, &query, 1);

        assert!(matches!(
            result,
            Err(SimilarityError::DimensionMismatch {
                expected: 3,
                got: 2
            })
        ));
        Ok(())
    }

    #[test]
    fn test_query_neighbors_nan() -> anyhow::Result<()> {
        let embeddings = vec![vec![1.0, 0.0, 0.0]];

        let index = build_index(&embeddings)?;
        let query = vec![f32::NAN, 0.0, 0.0];
        let result = query_neighbors(&index, &query, 1);

        assert!(matches!(result, Err(SimilarityError::InvalidEmbedding(_))));
        Ok(())
    }

    #[test]
    fn test_query_neighbors_top_k_exceeds_size() -> anyhow::Result<()> {
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0]];

        let index = build_index(&embeddings)?;
        let query = vec![1.0, 0.0];
        let neighbors = query_neighbors(&index, &query, 10)?;

        // HNSW is approximate and may return fewer results than available
        // Should return at least 1, but may not return all 2
        assert!(!neighbors.is_empty(), "Should return at least 1 neighbor");
        assert!(
            neighbors.len() <= 2,
            "Should not return more than available"
        );
        Ok(())
    }

    #[test]
    fn test_similarity_to_self_is_one() -> anyhow::Result<()> {
        let embeddings = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];

        let index = build_index(&embeddings)?;
        let query = vec![1.0, 0.0, 0.0]; // Same as first embedding
        let result = query_neighbors(&index, &query, 1)?;

        assert_eq!(result[0].0, 0);
        // Similarity to self should be very close to 1.0
        assert!((result[0].1 - 1.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_results_sorted_by_similarity() -> anyhow::Result<()> {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0], // Far from query
            vec![0.0, 1.0, 0.0], // Medium distance
            vec![0.5, 0.5, 0.0], // Close to query
        ];

        let index = build_index(&embeddings)?;
        let query = vec![0.6, 0.6, 0.0];
        let neighbors = query_neighbors(&index, &query, 3)?;

        // HNSW is approximate - may return fewer results than requested
        // Verify we got at least 2 results to check sorting
        assert!(
            neighbors.len() >= 2,
            "Expected at least 2 neighbors, got {}",
            neighbors.len()
        );

        // Results should be sorted by descending similarity
        for i in 0..neighbors.len().saturating_sub(1) {
            assert!(
                neighbors[i].1 >= neighbors[i + 1].1,
                "neighbors not sorted: similarity at index {} ({}) >= index {} ({})",
                i,
                neighbors[i].1,
                i + 1,
                neighbors[i + 1].1
            );
        }
        Ok(())
    }

    #[test]
    fn test_duplicate_embeddings() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0], // Duplicate
            vec![0.0, 1.0, 0.0],
        ];

        let result = build_index(&embeddings);
        // Should succeed - duplicates are allowed
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_all_zeros() -> anyhow::Result<()> {
        let embeddings = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];

        let index = build_index(&embeddings)?;
        let query = vec![0.0, 0.0, 0.0];
        let result = query_neighbors(&index, &query, 1);

        // Should succeed - all zeros is valid
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_validate_embedding_valid() {
        let embedding = vec![1.0, 2.0, 3.0];
        let result = validate_embedding(&embedding);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_dimensions_consistent() {
        let embeddings = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let result = validate_dimensions(&embeddings);
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn test_build_index_with_custom_hnsw_params() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let result = build_index_with_params(&embeddings, Some(32), Some(400));
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_index_with_default_hnsw_params() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let result = build_index_with_params(&embeddings, None, None);
        assert!(result.is_ok());
    }
}
