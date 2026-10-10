// Ikke et ekstra konsollvindu på Windows i ferdige bygg.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kerygma_app::run()
}
