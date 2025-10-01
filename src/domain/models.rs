use petgraph::stable_graph::{NodeIndex, StableGraph};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeData {
    pub x: f64,
    pub y: f64,
    pub elev: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdgeData {
    pub id: u64,
    pub length: f64,
    pub climb: f64,
    pub slope: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoadGraph {
    pub graph: StableGraph<NodeData, EdgeData>,
    pub node_map: HashMap<u64, NodeIndex>,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub points: Vec<(f64, f64)>,  // (cum_dist, rel_elev), sorted, starts with (0.0, 0.0)
}

impl Profile {
    pub fn new(mut points: Vec<(f64, f64)>) -> Self {
        if points.is_empty() || points[0].0 != 0.0 || points[0].1 != 0.0 {
            points.insert(0, (0.0, 0.0));
        }
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        Self { points }
    }

    pub fn total_length(&self) -> f64 {
        self.points.last().map(|p| p.0).unwrap_or(0.0)
    }

    pub fn interpolate(&self, s: f64) -> f64 {
        if s <= 0.0 {
            return 0.0;
        }
        let mut prev = &self.points[0];
        for curr in &self.points[1..] {
            if s <= curr.0 {
                let t = (s - prev.0) / (curr.0 - prev.0);
                return prev.1 + t * (curr.1 - prev.1);
            }
            prev = curr;
        }
        prev.1
    }
}

#[derive(Clone, Debug)]
pub struct Query {
    pub c: (f64, f64),
    pub d: f64,
    pub p: Profile,
}

#[derive(Clone, Debug)]
pub struct Route {
    pub si: f64,
    pub ti: f64,
    pub edge_ids: Vec<u64>,
}