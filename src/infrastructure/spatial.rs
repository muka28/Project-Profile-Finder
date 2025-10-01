use rstar::{AABB, PointDistance, RTreeObject};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SpatialEdge {
    pub p_u: [f64; 2],
    pub p_v: [f64; 2],
    pub u: petgraph::stable_graph::NodeIndex,
    pub v: petgraph::stable_graph::NodeIndex,
    pub e_idx: petgraph::stable_graph::EdgeIndex,
    pub length: f64,
    pub climb: f64,
    pub slope: f64,
    pub id: u64,
}

impl RTreeObject for SpatialEdge {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_points(&[self.p_u, self.p_v])
    }
}

impl PointDistance for SpatialEdge {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        distance_to_point(point, self).powi(2)
    }
}

pub fn distance_to_point(point: &[f64; 2], se: &SpatialEdge) -> f64 {
    let (proj, _) = project_point_to_segment(point, se);
    (proj[0] - point[0]).powi(2) + (proj[1] - point[1]).powi(2).sqrt()
}

pub fn project_point_to_segment(point: &[f64; 2], se: &SpatialEdge) -> ([f64; 2], f64) {
    let a = point[0] - se.p_u[0];
    let b = point[1] - se.p_u[1];
    let c = se.p_v[0] - se.p_u[0];
    let d = se.p_v[1] - se.p_u[1];
    let dot = a * c + b * d;
    let len_sq = c * c + d * d;
    let param = if len_sq != 0.0 { dot / len_sq } else { -1.0 };
    let (xx, yy) = if param < 0.0 {
        (se.p_u[0], se.p_u[1])
    } else if param > 1.0 {
        (se.p_v[0], se.p_v[1])
    } else {
        (se.p_u[0] + param * c, se.p_u[1] + param * d)
    };
    ([xx, yy], param.max(0.0).min(1.0))
}