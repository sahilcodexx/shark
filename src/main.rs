//! Main entry point for the Shark file manager application.

use shark::SharkApp;

fn main() -> glib::ExitCode {
    let app = SharkApp::new();
    app.run()
}
