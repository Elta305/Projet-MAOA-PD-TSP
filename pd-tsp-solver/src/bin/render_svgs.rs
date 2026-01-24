use std::fs;
use std::path::Path;
use pd_tsp_solver::visualization::Visualizer;

fn main() {
    let figs = Path::new("report").join("figs");
    if !figs.exists() {
        eprintln!("report/figs not found");
        std::process::exit(1);
    }

    for entry in fs::read_dir(&figs).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "svg" || ext == "svg\r" {
                let svg = match fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(e) => { eprintln!("Failed to read {:?}: {}", path, e); continue; }
                };
                let out = path.with_extension("png");
                match Visualizer::svg_to_png_file(&svg, &out) {
                    Ok(()) => println!("Converted {:?} -> {:?}", path.file_name().unwrap(), out.file_name().unwrap()),
                    Err(e) => eprintln!("Failed to convert {:?}: {}", path.file_name().unwrap(), e),
                }
            }
        }
    }
}
