use anyhow::Result;
use clap::Parser;
use project_profile_finder::application::find_route;
use project_profile_finder::domain::{Profile, Query};
use project_profile_finder::infrastructure::{load_data, AppData};
use std::path::PathBuf;
use plotters::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about = "Visualize routes and elevation profiles")]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(long, help = "Center X coordinate")]
    cx: f64,
    #[arg(long, help = "Center Y coordinate")]
    cy: f64,
    #[arg(short, long, help = "Search radius")]
    distance: f64,
    #[arg(long, help = "Profile points as comma-separated pairs: d1,z1,d2,z2,...")]
    profile: String,
    #[arg(short, long, default_value = "route_map.png", help = "Output map filename")]
    map_output: PathBuf,
    #[arg(short, long, default_value = "elevation_profile.png", help = "Output profile filename")]
    profile_output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data = load_data(&args.input)?;

    // Parse profile points
    let profile_parts: Vec<f64> = args.profile
        .split(',')
        .map(|s| s.trim().parse())
        .collect::<Result<Vec<_>, _>>()?;

    if profile_parts.len() % 2 != 0 {
        return Err(anyhow::anyhow!("Profile points must be in pairs (distance, elevation)"));
    }

    let mut profile_points = Vec::new();
    for chunk in profile_parts.chunks(2) {
        profile_points.push((chunk[0], chunk[1]));
    }

    let target_profile = Profile::new(profile_points);
    let query = Query {
        c: (args.cx, args.cy),
        d: args.distance,
        p: target_profile.clone(),
    };

    println!("Searching for route near ({}, {}) within {}m radius", args.cx, args.cy, args.distance);
    println!("Target profile length: {:.1}m", target_profile.total_length());

    match find_route(&data, &query)? {
        Some(route) => {
            println!("Found route with {} edges", route.edge_ids.len());
            println!("Route segments: si={:.3}, ti={:.3}, edges: {:?}",
                     route.si, route.ti, route.edge_ids);

            // Extract actual route profile
            let actual_profile = extract_route_profile(&data, &route)?;

            // Create visualizations
            create_map_visualization(&data, &query, &route, &args.map_output)?;
            create_profile_comparison(&target_profile, &actual_profile, &args.profile_output)?;

            println!("Map saved to: {:?}", args.map_output);
            println!("Profile comparison saved to: {:?}", args.profile_output);
        }
        None => {
            println!("No feasible route found within tolerance");

            // Still create map showing search area
            create_search_area_visualization(&data, &query, &args.map_output)?;
            println!("Search area map saved to: {:?}", args.map_output);
        }
    }

    Ok(())
}

fn extract_route_profile(data: &AppData, route: &project_profile_finder::domain::Route) -> Result<Profile> {
    let mut points = vec![(0.0, 0.0)];
    let mut cumulative_distance = 0.0;
    let mut cumulative_elevation = 0.0;

    for (i, &edge_id) in route.edge_ids.iter().enumerate() {
        // Find the edge in the graph
        let mut edge_data = None;
        for e_idx in data.graph.graph.edge_indices() {
            if data.graph.graph[e_idx].id == edge_id {
                edge_data = Some(&data.graph.graph[e_idx]);
                break;
            }
        }

        let edge = edge_data.ok_or_else(|| anyhow::anyhow!("Edge {} not found", edge_id))?;

        let (length, climb) = if i == 0 && i == route.edge_ids.len() - 1 {
            // Single edge, use both si and ti
            let total_fraction = route.ti - route.si;
            (edge.length * total_fraction, edge.climb * total_fraction)
        } else if i == 0 {
            // First edge, use from si to end
            let fraction = 1.0 - route.si;
            (edge.length * fraction, edge.climb * fraction)
        } else if i == route.edge_ids.len() - 1 {
            // Last edge, use from start to ti
            (edge.length * route.ti, edge.climb * route.ti)
        } else {
            // Middle edge, use entirely
            (edge.length, edge.climb)
        };

        cumulative_distance += length;
        cumulative_elevation += climb;
        points.push((cumulative_distance, cumulative_elevation));
    }

    Ok(Profile { points })
}

fn create_map_visualization(
    data: &AppData,
    query: &Query,
    route: &project_profile_finder::domain::Route,
    output_path: &PathBuf,
) -> Result<()> {
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find bounds of the graph
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for node in data.graph.graph.node_weights() {
        min_x = min_x.min(node.x);
        max_x = max_x.max(node.x);
        min_y = min_y.min(node.y);
        max_y = max_y.max(node.y);
    }

    // Add some padding
    let padding = 20.0;
    min_x -= padding;
    max_x += padding;
    min_y -= padding;
    max_y += padding;

    let mut chart = ChartBuilder::on(&root)
        .caption("Route Map", ("sans-serif", 30))
        .margin(5)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    // Draw all edges in light gray
    for e_idx in data.graph.graph.edge_indices() {
        let (u, v) = data.graph.graph.edge_endpoints(e_idx).unwrap();
        let node_u = &data.graph.graph[u];
        let node_v = &data.graph.graph[v];

        chart.draw_series(LineSeries::new(
            vec![(node_u.x, node_u.y), (node_v.x, node_v.y)],
            &RGBColor(128, 128, 128).mix(0.3),
        ))?;
    }

    // Draw search circle
    let circle_points: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::PI / 180.0;
            (
                query.c.0 + query.d * angle.cos(),
                query.c.1 + query.d * angle.sin(),
            )
        })
        .collect();

    chart.draw_series(LineSeries::new(circle_points, BLUE.mix(0.5)))?
        .label("Search Area")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], BLUE));

    // Draw the found route in red
    for &edge_id in &route.edge_ids {
        for e_idx in data.graph.graph.edge_indices() {
            if data.graph.graph[e_idx].id == edge_id {
                let (u, v) = data.graph.graph.edge_endpoints(e_idx).unwrap();
                let node_u = &data.graph.graph[u];
                let node_v = &data.graph.graph[v];

                chart.draw_series(LineSeries::new(
                    vec![(node_u.x, node_u.y), (node_v.x, node_v.y)],
                    RED.stroke_width(3),
                ))?;
                break;
            }
        }
    }

    // Mark center point
    chart.draw_series(PointSeries::of_element(
        vec![(query.c.0, query.c.1)],
        5,
        GREEN,
        &|c, s, st| {
            EmptyElement::at(c) + Circle::new((0, 0), s, st.filled())
        },
    ))?
    .label("Center")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], GREEN));

    chart.configure_series_labels().draw()?;
    root.present()?;

    Ok(())
}

fn create_search_area_visualization(
    data: &AppData,
    query: &Query,
    output_path: &PathBuf,
) -> Result<()> {
    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find bounds
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for node in data.graph.graph.node_weights() {
        min_x = min_x.min(node.x);
        max_x = max_x.max(node.x);
        min_y = min_y.min(node.y);
        max_y = max_y.max(node.y);
    }

    let padding = 20.0;
    min_x -= padding;
    max_x += padding;
    min_y -= padding;
    max_y += padding;

    let mut chart = ChartBuilder::on(&root)
        .caption("Search Area (No Route Found)", ("sans-serif", 30))
        .margin(5)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    // Draw all edges
    for e_idx in data.graph.graph.edge_indices() {
        let (u, v) = data.graph.graph.edge_endpoints(e_idx).unwrap();
        let node_u = &data.graph.graph[u];
        let node_v = &data.graph.graph[v];

        chart.draw_series(LineSeries::new(
            vec![(node_u.x, node_u.y), (node_v.x, node_v.y)],
            RGBColor(128, 128, 128).mix(0.5),
        ))?;
    }

    // Draw search circle
    let circle_points: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let angle = i as f64 * std::f64::consts::PI / 180.0;
            (
                query.c.0 + query.d * angle.cos(),
                query.c.1 + query.d * angle.sin(),
            )
        })
        .collect();

    chart.draw_series(LineSeries::new(circle_points, RED.mix(0.7)))?;

    // Mark center
    chart.draw_series(PointSeries::of_element(
        vec![(query.c.0, query.c.1)],
        5,
        RED,
        &|c, s, st| {
            EmptyElement::at(c) + Circle::new((0, 0), s, st.filled())
        },
    ))?;

    root.present()?;
    Ok(())
}

fn create_profile_comparison(
    target: &Profile,
    actual: &Profile,
    output_path: &PathBuf,
) -> Result<()> {
    let root = BitMapBackend::new(output_path, (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_distance = target.total_length().max(actual.total_length());
    let mut min_elev = f64::INFINITY;
    let mut max_elev = f64::NEG_INFINITY;

    // Find elevation bounds
    for point in &target.points {
        min_elev = min_elev.min(point.1);
        max_elev = max_elev.max(point.1);
    }
    for point in &actual.points {
        min_elev = min_elev.min(point.1);
        max_elev = max_elev.max(point.1);
    }

    let elev_padding = (max_elev - min_elev) * 0.1;
    min_elev -= elev_padding;
    max_elev += elev_padding;

    let mut chart = ChartBuilder::on(&root)
        .caption("Elevation Profile Comparison", ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..max_distance, min_elev..max_elev)?;

    chart
        .configure_mesh()
        .x_desc("Distance (m)")
        .y_desc("Elevation (m)")
        .draw()?;

    // Draw target profile
    chart
        .draw_series(LineSeries::new(
            target.points.iter().cloned(),
            BLUE.stroke_width(2),
        ))?
        .label("Target Profile")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], BLUE));

    // Draw actual profile
    chart
        .draw_series(LineSeries::new(
            actual.points.iter().cloned(),
            RED.stroke_width(2),
        ))?
        .label("Actual Route")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], RED));

    // Mark key points
    chart.draw_series(PointSeries::of_element(
        target.points.iter().cloned(),
        3,
        BLUE,
        &|c, s, st| {
            EmptyElement::at(c) + Circle::new((0, 0), s, st.filled())
        },
    ))?;

    chart.draw_series(PointSeries::of_element(
        actual.points.iter().cloned(),
        3,
        RED,
        &|c, s, st| {
            EmptyElement::at(c) + Circle::new((0, 0), s, st.filled())
        },
    ))?;

    chart.configure_series_labels().draw()?;
    root.present()?;

    Ok(())
}