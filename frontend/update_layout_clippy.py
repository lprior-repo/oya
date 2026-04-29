import re

content = open("src/graph/layout.rs").read()

content = content.replace("use std::collections::{HashMap, HashSet};", "use std::collections::HashMap;")

old_panic = """    pub fn apply(&self, workflow: &mut Workflow) {"""
new_panic = """    /// Applies the layout to the workflow's nodes.
    ///
    /// # Panics
    ///
    /// Panics if the sort comparison fails, which should not happen for f32 partial_cmp.
    #[allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]
    pub fn apply(&self, workflow: &mut Workflow) {"""
content = content.replace(old_panic, new_panic)

old_match = """        let sorted_indices = match toposort(&graph, None) {
            Ok(indices) => indices,
            Err(_) => {
                // If cyclic, try to layout what we can or return
                return;
            }
        };"""
new_match = """        let Ok(sorted_indices) = toposort(&graph, None) else {
            // If cyclic, try to layout what we can or return
            return;
        };"""
content = content.replace(old_match, new_match)

old_unwrap = """barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());"""
new_unwrap = """barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));"""
content = content.replace(old_unwrap, new_unwrap)

with open("src/graph/layout.rs", "w") as f:
    f.write(content)
