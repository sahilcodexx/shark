//! Main entry point for the Shark file manager application.

use shark::SharkApp;
use std::path::PathBuf;

fn main() {
    let folder = std::env::args().nth(1).map(PathBuf::from);
    SharkApp::run(folder);
}
