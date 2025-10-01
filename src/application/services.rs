use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableGraph};
use rstar::RTree;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::domain::{integral_abs_diff, AreaMatcher, EdgeData, NodeData, Profile, ProfileMatcher, Query, RoadGraph, Route};
use crate::infrastructure::{AppData, SpatialEdge, project_point_to_segment, distance_to_point};

#[derive(Clone)]
struct PartialPath {
    node: NodeIndex,
    length: f64,
    cum_area: f64,
    rel_elev: f64,
    path: Vec<(EdgeIndex, f64)>,  // (edge_idx, fraction_end)
    first_fraction: f64,
    first_edge_idx: Option<EdgeIndex>,
}

pub fn build_graph_from_jsonl(path: &Path) -> Result<RoadGraph> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut graph = StableGraph::<NodeData, EdgeData>::new();
    let mut node_map = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let record: serde_json::Value = serde_json::from_str(&line)?;
        match record["type"].as_str() {
            Some("meta") => {},  // Ignore
            Some("node") => {
                let id = record["id"].as_u64().ok_or(anyhow!("Invalid node id"))?;
                let x = record["x"].as_f64().ok_or(anyhow!("Invalid x"))?;
                let y = record["y"].as_f64().ok_or(anyhow!("Invalid y"))?;
                let elev = record["elev"].as_f64().ok_or(anyhow!("Invalid elev"))?;
                let idx = graph.add_node(NodeData { x, y, elev });
                node_map.insert(id, idx);
            }
            Some("edge") => {
                let id = record["id"].as_u64().ok_or(anyhow!("Invalid edge id"))?;
                let u = record["u"].as_u64().ok_or(anyhow!("Invalid u"))?;
                let v = record["v"].as_u64().ok_or(anyhow!("Invalid v"))?;
                let length = record["length_m"].as_f64().ok_or(anyhow!("Invalid length"))?;
                let climb = record["climb_m"].as_f64().ok_or(anyhow!("Invalid climb"))?;
                let slope = record["slope"].as_f64().ok_or(anyhow!("Invalid slope"))?;
                let u_idx = *node_map.get(&u).ok_or(anyhow!("Unknown u"))?;
                let v_idx = *node_map.get(&v).ok_or(anyhow!("Unknown v"))?;
                let edge = EdgeData { id, length, climb, slope };
                graph.add_edge(u_idx, v_idx, edge);
            }
            _ => return Err(anyhow!("Unknown record type")),
        }
    }
    Ok(RoadGraph { graph, node_map })
}

pub fn build_spatial_index(graph: &RoadGraph) -> RTree<SpatialEdge> {
    let mut spatial_edges = Vec::new();
    for e_idx in graph.graph.edge_indices() {
        let (u, v) = graph.graph.edge_endpoints(e_idx).unwrap();
        let node_u = &graph.graph[u];
        let node_v = &graph.graph[v];
        let edge = &graph.graph[e_idx];
        spatial_edges.push(SpatialEdge {
            p_u: [node_u.x, node_u.y],
            p_v: [node_v.x, node_v.y],
            u,
            v,
            e_idx,
            length: edge.length,
            climb: edge.climb,
            slope: edge.slope,
            id: edge.id,
        });
    }
    RTree::bulk_load(spatial_edges)
}

pub fn find_route(data: &AppData, query: &Query) -> Result<Option<Route>> {
    let l = query.p.total_length();
    if l == 0.0 {
        return Ok(None);
    }
    let eps = 5.0f64.max(0.05 * l);
    // Find candidate starts: edges within D
    let bound_box = rstar::AABB::from_corners([query.c.0 - query.d, query.c.1 - query.d], [query.c.0 + query.d, query.c.1 + query.d]);
    let candidates: Vec<&SpatialEdge> = data.rtree.locate_in_envelope(&bound_box).collect();
    let mut start_partials = Vec::new();
    for se in candidates {
        let dist = distance_to_point(&[query.c.0, query.c.1], se);
        if dist > query.d {
            continue;
        }
        let (_proj, fraction) = project_point_to_segment(&[query.c.0, query.c.1], se);
        let partial_len = (1.0 - fraction) * se.length;
        let partial_climb = (1.0 - fraction) * se.climb;
        let area = integral_abs_diff(partial_len, 0.0 - query.p.interpolate(0.0), partial_climb - query.p.interpolate(partial_len));
        start_partials.push(PartialPath {
            node: se.v,
            length: partial_len,
            cum_area: area,
            rel_elev: partial_climb,
            path: vec![],
            first_fraction: fraction,
            first_edge_idx: Some(se.e_idx),
        });
    }
    if start_partials.is_empty() {
        return Ok(None);
    }
    // Beam search from each start, but to optimize, start from all in initial beam
    let beam_width = 50;
    let mut beam: Vec<PartialPath> = start_partials;
    let mut best: Option<(f64, PartialPath)> = None;
    let max_steps = (2.0 * l / 50.0) as usize;  // Assume avg edge 50m
    for _step in 0..max_steps {
        if beam.is_empty() {
            break;
        }
        let mut next_beam = Vec::new();
        for path in beam {
            if path.length > l + eps {
                continue;
            }
            if (path.length - l).abs() <= eps {
                // Compute final score with offset
                let matcher = AreaMatcher { use_offset: true };
                let actual_profile = extract_profile(&path, data);  // Defined below
                let score = matcher.score(&actual_profile, &query.p);
                if let Some((best_score, _)) = &best {
                    if score < *best_score {
                        best = Some((score, path.clone()));
                    }
                } else {
                    best = Some((score, path.clone()));
                }
            }
            // Extend
            for n_e in data.graph.graph.neighbors(path.node) {
                let e_idx = data.graph.graph.find_edge(path.node, n_e).unwrap();
                let edge = &data.graph.graph[e_idx];
                let new_len = path.length + edge.length;
                if new_len > l + eps * 2.0 {
                    continue;
                }
                let new_rel = path.rel_elev + edge.climb;
                let area_add = integral_abs_diff(edge.length, path.rel_elev - query.p.interpolate(path.length), new_rel - query.p.interpolate(new_len));
                let new_area = path.cum_area + area_add;
                let mut new_path = path.path.clone();
                new_path.push((e_idx, 1.0));
                next_beam.push(PartialPath {
                    node: n_e,
                    length: new_len,
                    cum_area: new_area,
                    rel_elev: new_rel,
                    path: new_path,
                    first_fraction: path.first_fraction,
                    first_edge_idx: path.first_edge_idx,
                });
            }
        }
        // Sort by estimated full score, keep top
        next_beam.sort_by(|a, b| {
            let est_a = if a.length > 0.0 && a.cum_area.is_finite() {
                a.cum_area / a.length * l
            } else {
                f64::INFINITY
            };
            let est_b = if b.length > 0.0 && b.cum_area.is_finite() {
                b.cum_area / b.length * l
            } else {
                f64::INFINITY
            };
            est_a.partial_cmp(&est_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        beam = next_beam.into_iter().take(beam_width).collect();
    }
    // Add any remaining in tolerance
    for path in beam {
        if (path.length - l).abs() <= eps {
            let matcher = AreaMatcher { use_offset: true };
            let actual_profile = extract_profile(&path, data);
            let score = matcher.score(&actual_profile, &query.p);
            if let Some((best_score, _)) = &best {
                if score < *best_score {
                    best = Some((score, path));
                }
            } else {
                best = Some((score, path));
            }
        }
    }
    if let Some((_, best_path)) = best {
    let mut edge_ids = Vec::new();
    if let Some(first_idx) = best_path.first_edge_idx {
        let first_edge = &data.graph.graph[first_idx];
        edge_ids.push(first_edge.id);
    }
    // Use .iter() to borrow instead of moving
    for (e_idx, _) in best_path.path.iter() {
        let edge = &data.graph.graph[*e_idx];
        edge_ids.push(edge.id);
    }
    let ti = if let Some(last) = best_path.path.last() {
        last.1
    } else {
        1.0 - best_path.first_fraction  // If only partial first
    };
    Ok(Some(Route {
        si: best_path.first_fraction,
        ti,
        edge_ids,
    }))
} else {
    Ok(None)
}
}

fn extract_profile(path: &PartialPath, data: &AppData) -> Profile {
    let mut points = vec![(0.0, 0.0)];
    let mut s = 0.0;
    let mut rel = 0.0;
    if path.first_edge_idx.is_some() {
        let first_idx = path.first_edge_idx.unwrap();
        let first_edge = &data.graph.graph[first_idx];
        let partial_len = (1.0 - path.first_fraction) * first_edge.length;
        let partial_climb = (1.0 - path.first_fraction) * first_edge.climb;
        s += partial_len;
        rel += partial_climb;
        points.push((s, rel));
    }
    for (e_idx, frac) in &path.path {
        let edge = &data.graph.graph[*e_idx];
        let this_len = *frac * edge.length;
        let this_climb = *frac * edge.climb;
        s += this_len;
        rel += this_climb;
        points.push((s, rel));
    }
    Profile { points }
}