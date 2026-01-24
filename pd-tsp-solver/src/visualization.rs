//! Visualization utilities for PD-TSP solutions.
//! 
//! Generates SVG visualizations of tours and exports for plotting.

use crate::instance::PDTSPInstance;
use crate::solution::Solution;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
#[cfg(feature = "resvg")]
use resvg::usvg;
#[cfg(feature = "resvg")]
use resvg::render;
#[cfg(feature = "resvg")]
use resvg::FitTo;
#[cfg(feature = "resvg")]
use resvg::tiny_skia::{Pixmap, Transform};
#[cfg(feature = "resvg")]
use resvg::usvg::TreeParsing;

/// SVG visualization generator
pub struct Visualizer {
    /// Canvas width
    pub width: f64,
    /// Canvas height  
    pub height: f64,
    /// Margin
    pub margin: f64,
    /// Node radius
    pub node_radius: f64,
}

impl Default for Visualizer {
    fn default() -> Self {
        Visualizer {
            width: 800.0,
            height: 800.0,
            margin: 50.0,
            node_radius: 8.0,
        }
    }
}

impl Visualizer {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Generate SVG visualization of a solution
    pub fn generate_svg(&self, instance: &PDTSPInstance, solution: &Solution) -> String {
        let mut svg = String::new();
        
        let (min_x, max_x, min_y, max_y) = self.get_bounds(instance);
        
        let scale_x = (self.width - 2.0 * self.margin) / (max_x - min_x).max(1.0);
        let scale_y = (self.height - 2.0 * self.margin) / (max_y - min_y).max(1.0);
        let scale = scale_x.min(scale_y);
        
        svg.push_str(&format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<style>
    .node {{ fill: #3498db; stroke: #2c3e50; stroke-width: 2; }}
    .depot {{ fill: #e74c3c; stroke: #c0392b; stroke-width: 2; }}
    .pickup {{ fill: #2ecc71; stroke: #27ae60; stroke-width: 2; }}
    .delivery {{ fill: #f39c12; stroke: #d68910; stroke-width: 2; }}
    .edge {{ stroke: #34495e; stroke-width: 2; fill: none; }}
    .label {{ font-family: Arial; font-size: 10px; fill: #2c3e50; }}
    .title {{ font-family: Arial; font-size: 14px; fill: #2c3e50; font-weight: bold; }}
</style>
<rect width="100%" height="100%" fill="#ecf0f1"/>
"##,
            self.width, self.height, self.width, self.height
        ));
        
        svg.push_str(&format!(
            r##"<text x="{}" y="25" class="title">Instance: {} | Cost: {:.2} | Feasible: {}</text>
"##,
            self.margin, instance.name, solution.cost, solution.feasible
        ));
        
        let transform = |x: f64, y: f64| -> (f64, f64) {
            let tx = self.margin + (x - min_x) * scale;
            let ty = self.height - self.margin - (y - min_y) * scale;
            (tx, ty)
        };
        
        if solution.tour.len() > 1 {
            for i in 0..solution.tour.len() {
                let from = solution.tour[i];
                let to = solution.tour[(i + 1) % solution.tour.len()];
                
                let (x1, y1) = transform(instance.nodes[from].x, instance.nodes[from].y);
                let (x2, y2) = transform(instance.nodes[to].x, instance.nodes[to].y);
                
                svg.push_str(&format!(
                    r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" class="edge" marker-end="url(#arrow)"/>
"#,
                    x1, y1, x2, y2
                ));
            }
        }
        
        svg.push_str(r##"<defs>
<marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto" markerUnits="strokeWidth">
<path d="M0,0 L0,6 L9,3 z" fill="#34495e"/>
</marker>
</defs>
"##);
        
        for node in &instance.nodes {
            let (x, y) = transform(node.x, node.y);
            
            let class = if node.id == 0 {
                "depot"
            } else if node.demand < 0 {
                "pickup"
            } else if node.demand > 0 {
                "delivery"
            } else {
                "node"
            };
            
            svg.push_str(&format!(
                r##"<circle cx="{:.2}" cy="{:.2}" r="{}" class="{}"/>
"##,
                x, y, self.node_radius, class
            ));
            
            svg.push_str(&format!(
                r##"<text x="{:.2}" y="{:.2}" class="label" text-anchor="middle">{}</text>
"##,
                x, y - self.node_radius - 3.0, node.id
            ));
        }
        
        let legend_y = self.height - 30.0;
        svg.push_str(&format!(r##"
<rect x="{}" y="{}" width="15" height="15" class="depot"/>
<text x="{}" y="{}" class="label">Depot</text>
<rect x="{}" y="{}" width="15" height="15" class="pickup"/>
<text x="{}" y="{}" class="label">Pickup</text>
<rect x="{}" y="{}" width="15" height="15" class="delivery"/>
<text x="{}" y="{}" class="label">Delivery</text>
"##,
            self.margin, legend_y, self.margin + 20.0, legend_y + 12.0,
            self.margin + 80.0, legend_y, self.margin + 100.0, legend_y + 12.0,
            self.margin + 160.0, legend_y, self.margin + 180.0, legend_y + 12.0
        ));
        
        svg.push_str("</svg>");
        
        svg
    }
    
    /// Generate load profile SVG
    pub fn generate_load_profile_svg(&self, instance: &PDTSPInstance, solution: &Solution) -> String {
        let load_profile = solution.load_profile(instance);
        let mut svg = String::new();
        
        let width = self.width;
        let height = 300.0;
        let margin = 50.0;
        
        svg.push_str(&format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<style>
    .line {{ stroke: #3498db; stroke-width: 2; fill: none; }}
    .capacity {{ stroke: #e74c3c; stroke-width: 1; stroke-dasharray: 5,5; }}
    .axis {{ stroke: #2c3e50; stroke-width: 1; }}
    .label {{ font-family: Arial; font-size: 12px; fill: #2c3e50; }}
    .title {{ font-family: Arial; font-size: 14px; fill: #2c3e50; font-weight: bold; }}
</style>
<rect width="100%" height="100%" fill="#ecf0f1"/>
"##,
            width, height, width, height
        ));
        
        svg.push_str(&format!(
            r#"<text x="{}" y="25" class="title">Load Profile - Capacity: {}</text>
"#,
            margin, instance.capacity
        ));
        
        let plot_width = width - 2.0 * margin;
        let plot_height = height - 2.0 * margin;
        
        let x_scale = plot_width / load_profile.len().max(1) as f64;
        let max_load = load_profile.iter().map(|&l| l.abs()).max().unwrap_or(1);
        let y_max = instance.capacity.max(max_load) as f64;
        let y_scale = plot_height / (2.0 * y_max);
        let y_center = margin + plot_height / 2.0;
        
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" class="axis"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" class="axis"/>
"##,
            margin, y_center, width - margin, y_center,
            margin, margin, margin, height - margin
        ));
        
        let cap_y_top = y_center - instance.capacity as f64 * y_scale;
        let cap_y_bottom = y_center + instance.capacity as f64 * y_scale;
        svg.push_str(&format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" class="capacity"/>
<line x1="{}" y1="{}" x2="{}" y2="{}" class="capacity"/>
<text x="{}" y="{}" class="label">+{}</text>
<text x="{}" y="{}" class="label">-{}</text>
"##,
            margin, cap_y_top, width - margin, cap_y_top,
            margin, cap_y_bottom, width - margin, cap_y_bottom,
            width - margin + 5.0, cap_y_top + 5.0, instance.capacity,
            width - margin + 5.0, cap_y_bottom + 5.0, instance.capacity
        ));
        
        let mut path = String::new();
        for (i, &load) in load_profile.iter().enumerate() {
            let x = margin + i as f64 * x_scale;
            let y = y_center - load as f64 * y_scale;
            
            if i == 0 {
                path.push_str(&format!("M {:.2} {:.2}", x, y));
            } else {
                path.push_str(&format!(" L {:.2} {:.2}", x, y));
            }
        }
        
        svg.push_str(&format!(r##"<path d="{}" class="line"/>
"##, path));
        
        for (i, &load) in load_profile.iter().enumerate() {
            let x = margin + i as f64 * x_scale;
            let y = y_center - load as f64 * y_scale;
            
            let color = if load.abs() > instance.capacity { "#e74c3c" } else { "#3498db" };
            svg.push_str(&format!(
                r##"<circle cx="{:.2}" cy="{:.2}" r="4" fill="{}"/>
"##,
                x, y, color
            ));
        }
        
        svg.push_str("</svg>");
        
        svg
    }
    
    /// Save SVG to file
    pub fn save_svg<P: AsRef<Path>>(&self, svg: &str, path: P) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(svg.as_bytes())?;
        Ok(())
    }

    /// Save SVG as PNG using an external converter if available.
    /// Tries `rsvg-convert`, then `magick convert`, then `inkscape`.
    pub fn save_png<P: AsRef<Path>>(&self, svg: &str, path: P) -> std::io::Result<()> {
        let path = path.as_ref();
        // Try native resvg renderer when the feature is enabled
        #[cfg(feature = "resvg")]
        {
            // parse
            let mut opt = usvg::Options::default();
            // keep default DPI and font dirs
            let rtree = usvg::Tree::from_str(svg, &opt).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("usvg parse error: {}", e)))?;
            // try to infer canvas size from SVG header (width/height attributes), fallback to 800x800
            let mut w = self.width as u32;
            let mut h = self.height as u32;
            if let Some(cap) = svg.split_once("width=\"") {
                if let Some(rest) = cap.1.split_once('"') {
                    if let Ok(v) = rest.0.parse::<f64>() { w = v as u32; }
                }
            }
            if let Some(cap) = svg.split_once("height=\"") {
                if let Some(rest) = cap.1.split_once('"') {
                    if let Ok(v) = rest.0.parse::<f64>() { h = v as u32; }
                }
            }
            let mut pixmap = Pixmap::new(w.max(1), h.max(1)).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Failed to create pixmap"))?;
            render(&rtree, FitTo::Original, Transform::default(), pixmap.as_mut()).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "resvg render failed"))?;
            pixmap.save_png(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("save_png failed: {}", e)))?;
            return Ok(());
        }

        // Fallback: write temporary svg and try external converters
        let tmp_svg = path.with_extension("svg.tmp");
        {
            let mut f = File::create(&tmp_svg)?;
            f.write_all(svg.as_bytes())?;
        }

        // Try rsvg-convert
        if let Ok(status) = Command::new("rsvg-convert").args(&["-o", path.to_string_lossy().as_ref(), tmp_svg.to_string_lossy().as_ref()]).status() {
            if status.success() {
                let _ = std::fs::remove_file(&tmp_svg);
                return Ok(());
            }
        }

        // Try ImageMagick `magick convert`
        if let Ok(status) = Command::new("magick").args(&["convert", tmp_svg.to_string_lossy().as_ref(), path.to_string_lossy().as_ref()]).status() {
            if status.success() {
                let _ = std::fs::remove_file(&tmp_svg);
                return Ok(());
            }
        }

        // Try inkscape
        if let Ok(status) = Command::new("inkscape").args(&[tmp_svg.to_string_lossy().as_ref(), "--export-type=png", "--export-filename", path.to_string_lossy().as_ref()]).status() {
            if status.success() {
                let _ = std::fs::remove_file(&tmp_svg);
                return Ok(());
            }
        }

        // Clean up and return error
        let _ = std::fs::remove_file(&tmp_svg);
        Err(std::io::Error::new(std::io::ErrorKind::Other, "No SVG->PNG converter succeeded (tried resvg, rsvg-convert, magick, inkscape)"))
    }

    /// Render an SVG string directly to PNG file using available renderer.
    pub fn svg_to_png_file(svg: &str, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(feature = "resvg")]
        {
            let mut opt = usvg::Options::default();
            let rtree = usvg::Tree::from_str(svg, &opt)?;
            // infer width/height from svg text
            let mut w = 800u32;
            let mut h = 800u32;
            if let Some(cap) = svg.split_once("width=\"") {
                if let Some(rest) = cap.1.split_once('"') {
                    if let Ok(v) = rest.0.parse::<f64>() { w = v as u32; }
                }
            }
            if let Some(cap) = svg.split_once("height=\"") {
                if let Some(rest) = cap.1.split_once('"') {
                    if let Ok(v) = rest.0.parse::<f64>() { h = v as u32; }
                }
            }
            let mut pixmap = Pixmap::new(w.max(1), h.max(1)).ok_or("Failed to create pixmap")?;
            render(&rtree, FitTo::Original, Transform::default(), pixmap.as_mut()).ok_or("resvg render failed")?;
            pixmap.save_png(out)?;
            return Ok(());
        }

        // Fallback: write svg to file and attempt external commands
        std::fs::write(out.with_extension("svg.tmp"), svg)?;
        let tmp = out.with_extension("svg.tmp");
        if Command::new("rsvg-convert").args(&["-o", out.to_string_lossy().as_ref(), tmp.to_string_lossy().as_ref()]).status().is_ok() {
            let _ = std::fs::remove_file(&tmp);
            return Ok(());
        }
        if Command::new("magick").args(&["convert", tmp.to_string_lossy().as_ref(), out.to_string_lossy().as_ref()]).status().is_ok() {
            let _ = std::fs::remove_file(&tmp);
            return Ok(());
        }
        if Command::new("inkscape").args(&[tmp.to_string_lossy().as_ref(), "--export-type=png", "--export-filename", out.to_string_lossy().as_ref()]).status().is_ok() {
            let _ = std::fs::remove_file(&tmp);
            return Ok(());
        }
        let _ = std::fs::remove_file(&tmp);
        Err("No converter available".into())
    }
    
    /// Get coordinate bounds
    fn get_bounds(&self, instance: &PDTSPInstance) -> (f64, f64, f64, f64) {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        
        for node in &instance.nodes {
            min_x = min_x.min(node.x);
            max_x = max_x.max(node.x);
            min_y = min_y.min(node.y);
            max_y = max_y.max(node.y);
        }
        
        (min_x, max_x, min_y, max_y)
    }
    
    /// Export data for external plotting (e.g., matplotlib)
    pub fn export_plot_data(&self, instance: &PDTSPInstance, solution: &Solution) -> String {
        let mut data = String::new();
        
        data.push_str("# PD-TSP Solution Data\n");
        data.push_str(&format!("# Instance: {}\n", instance.name));
        data.push_str(&format!("# Cost: {:.2}\n", solution.cost));
        data.push_str(&format!("# Feasible: {}\n\n", solution.feasible));
        
        data.push_str("# Nodes: id, x, y, demand\n");
        for node in &instance.nodes {
            data.push_str(&format!("{},{},{},{}\n", node.id, node.x, node.y, node.demand));
        }
        
        data.push_str("\n# Tour: sequence of node ids\n");
        let tour_str: Vec<String> = solution.tour.iter().map(|n| n.to_string()).collect();
        data.push_str(&tour_str.join(","));
        data.push('\n');
        
        data.push_str("\n# Load profile\n");
        let profile = solution.load_profile(instance);
        let profile_str: Vec<String> = profile.iter().map(|l| l.to_string()).collect();
        data.push_str(&profile_str.join(","));
        data.push('\n');
        
        data
    }
}

/// Generate comparison plot data for multiple solutions
pub fn generate_comparison_data(_instance: &PDTSPInstance, solutions: &[Solution]) -> String {
    let mut data = String::new();
    
    data.push_str("# Algorithm Comparison\n");
    data.push_str("algorithm,cost,time,feasible\n");
    
    for sol in solutions {
        data.push_str(&format!("{},{:.2},{:.4},{}\n",
            sol.algorithm, sol.cost, sol.computation_time, sol.feasible));
    }
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::Node;
    
    fn create_test_instance() -> PDTSPInstance {
        let nodes = vec![
            Node::new(0, 0.0, 0.0, 0, 0),
            Node::new(1, 1.0, 0.0, 5, 0),
            Node::new(2, 0.0, 1.0, -5, 0),
        ];
        
        use crate::instance::CostFunction;
        
        PDTSPInstance {
            cost_function: CostFunction::Distance,
            alpha: 0.1,
            beta: 0.5,
            name: "test".to_string(),
            comment: "test".to_string(),
            dimension: 3,
            capacity: 10,
            nodes,
            distance_matrix: vec![vec![0.0; 3]; 3],
            return_depot_demand: 0,
        }
    }
    
    #[test]
    fn test_visualizer() {
        let instance = create_test_instance();
        let solution = Solution::from_tour(&instance, vec![0, 1, 2], "test");
        
        let viz = Visualizer::new();
        let svg = viz.generate_svg(&instance, &solution);
        
        assert!(svg.contains("svg"));
        assert!(svg.contains("test"));
    }
}
