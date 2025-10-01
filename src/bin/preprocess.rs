use anyhow::Result;
use clap::Parser;
use project_profile_finder::application::{build_graph_from_jsonl, build_spatial_index};
use project_profile_finder::infrastructure::{save_data, AppData};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Preprocess road graph JSONL to binary")]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let graph = build_graph_from_jsonl(&args.input)?;
    let rtree = build_spatial_index(&graph);
    let data = AppData { graph, rtree };
    save_data(&data, &args.output)?;
    println!("Preprocessed data saved to {:?}", args.output);
    Ok(())
}