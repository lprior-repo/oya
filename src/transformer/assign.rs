use crate::analyze::Analysis;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tap::Pipe;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdMapping {
    pub id: String,
    pub filename: String,
    pub subcategory: String,
    pub slug: String,
}

pub fn assign_ids(analyses: Vec<Analysis>) -> (Vec<Analysis>, HashMap<String, IdMapping>) {
    let mut link_map = HashMap::new();
    let mut id_counts: HashMap<String, usize> = HashMap::new();

    for analysis in &analyses {
        let parts: Vec<&str> = analysis.source_path.split('/').collect();
        let subcategory = if parts.len() > 1 {
            parts
                .get(parts.len().saturating_sub(2))
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| "general".to_string())
        } else {
            "general".to_string()
        };

        let filename_stem = Path::new(&analysis.source_path)
            .file_stem()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        let mut slug = slugify(&filename_stem);

        let unique_key = format!("{}/{}/{}", analysis.category, subcategory, slug);
        let count = id_counts.entry(unique_key.clone()).or_insert(0);
        *count = count.saturating_add(1);

        if *count > 1 {
            slug = format!("{slug}-{count}");
        }

        let doc_id = format!("{}/{}/{}", analysis.category, subcategory, slug);
        let new_filename = format!("{}-{}-{}.md", analysis.category, subcategory, slug);

        link_map.insert(
            analysis.source_path.clone(),
            IdMapping {
                id: doc_id,
                filename: new_filename,
                subcategory,
                slug,
            },
        );
    }

    (analyses, link_map)
}

/// Generate a URL-safe slug using functional composition
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .pipe(|s| s.chars().take(40).collect())
}
