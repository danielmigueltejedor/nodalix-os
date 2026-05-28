mod app;
mod assets;
mod cad;
mod canvas;
mod canvas_fillet_chamfer;
mod canvas_grips;
mod canvas_hatch;
mod canvas_modify;
mod canvas_offset;
mod canvas_trim_extend;
mod document;
mod drawing;
mod export;
mod geometry;
mod import;
mod mesh;
mod reverse;
mod tool_parameters;
mod tools;
mod ui;
mod ui_context;
mod ui_history;
mod units;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--dwg-status") {
        println!("{}", import::dwg::dwg_status_report());
        return;
    }
    if args.get(1).map(String::as_str) == Some("--inspect-step") {
        let Some(path) = args.get(2) else {
            eprintln!("usage: nodalix-cad --inspect-step <file.step|file.stp>");
            std::process::exit(2);
        };
        let mut document = document::Document::new_empty();
        match import::step::import_step_metadata(std::path::Path::new(path), &mut document) {
            Ok(summary) => {
                println!("{} [{}]", summary.file_name, summary.format);
                println!("{}", summary.summary);
                for (label, value) in summary.details {
                    println!("{label}: {value}");
                }
                for warning in summary.warnings {
                    println!("warning: {warning}");
                }
            }
            Err(error) => {
                eprintln!("STEP inspection failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).map(String::as_str) == Some("--inspect-import") {
        let Some(path) = args.get(2) else {
            eprintln!("usage: nodalix-cad --inspect-import <cad-or-mesh-file>");
            std::process::exit(2);
        };
        let mut document = document::Document::new_empty();
        match import::import_path(std::path::Path::new(path), &mut document) {
            Ok(summary) => {
                println!("{} [{}]", summary.file_name, summary.format);
                println!("{}", summary.summary);
                for (label, value) in summary.details {
                    println!("{label}: {value}");
                }
                for warning in summary.warnings {
                    println!("warning: {warning}");
                }
            }
            Err(error) => {
                eprintln!("Import inspection failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    app::run();
}
