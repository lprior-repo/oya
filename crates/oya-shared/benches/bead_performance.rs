use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oya_shared::{Bead, BeadFilters};

fn benchmark_string_cloning(c: &mut Criterion) {
    // Create a large number of beads
    let beads: Vec<Bead> = (0..1000)
        .map(|i| {
            Bead::new(
                format!("bead-{}", i),
                format!(
                    "Task number {} with a very long title that contains lots of text",
                    i
                ),
            )
            .with_description(format!(
                "Description for bead {} with some additional text to make it longer",
                i
            ))
            .with_tag("important")
            .with_tag("urgent")
        })
        .collect();

    let search_term = "urgent";

    c.bench_function("search_with_string_cloning", |b| {
        b.iter(|| {
            beads
                .iter()
                .filter(|bead| bead.matches_search(search_term))
                .count()
        })
    });

    // Test filtering with different criteria
    let filters = BeadFilters {
        tag: Some("urgent".into()),
        ..Default::default()
    };

    c.bench_function("filter_by_tag", |b| {
        b.iter(|| beads.iter().filter(|bead| filters.matches(bead)).count())
    });

    // Test multiple filters
    let filters_multi = BeadFilters {
        tag: Some("important".into()),
        status: None,   // All statuses
        priority: None, // All priorities
    };

    c.bench_function("filter_multiple_criteria", |b| {
        b.iter(|| {
            beads
                .iter()
                .filter(|bead| filters_multi.matches(bead))
                .count()
        })
    });
}

fn benchmark_individual_field_access(c: &mut Criterion) {
    let bead = Bead::new("test-id-123", "Test Task Title")
        .with_description("This is a test description for performance benchmarking");

    c.bench_function("title_field_access", |b| {
        b.iter(|| {
            black_box(&bead.title);
            black_box(&bead.id);
            black_box(&bead.description);
        })
    });

    c.bench_function("title_string_clone", |b| {
        b.iter(|| {
            black_box(bead.title.to_string());
            black_box(bead.id.to_string());
            black_box(bead.description.to_string());
        })
    });
}

fn benchmark_large_collection_operations(c: &mut Criterion) {
    // Create a very large collection of beads
    let large_collection: Vec<Bead> = (0..5000)
        .map(|i| {
            Bead::new(
                format!("large-bead-{:05}", i),
                format!("Large task {} with extensive description text", i),
            )
            .with_description(format!("This is task number {} in a large collection. Description contains substantial text content for realistic benchmarking purposes.", i))
            .with_tag("large-collection")
            .with_dependency(format!("dep-{}", i % 100)) // Create some dependencies
        })
        .collect();

    // Test filtering performance
    c.bench_function("large_collection_filter", |b| {
        b.iter(|| {
            large_collection
                .iter()
                .filter(|bead| bead.matches_search("large"))
                .filter(|bead| bead.tags.contains(&"large-collection".into()))
                .count()
        })
    });

    // Test iteration and field access
    c.bench_function("large_collection_iteration", |b| {
        b.iter(|| {
            let mut total_length = 0;
            for bead in &large_collection {
                total_length += bead.title.len();
                total_length += bead.description.len();
                total_length += bead.id.len();
            }
            black_box(total_length);
        })
    });
}

criterion_group!(
    benches,
    benchmark_string_cloning,
    benchmark_individual_field_access,
    benchmark_large_collection_operations
);
criterion_main!(benches);
