// Import the engine components
#![windows_subsystem = "windows"]

mod engine;
mod network;
mod ui;
mod javascript_engine;

use engine::{TrussCore, EngineConfig, RenderingMode};
use network::NetworkManager;
use javascript_engine::ChronoScript;
use ui::Retro1996Browser;

fn main() {
    // Set up panic hook to handle crashes gracefully without showing console
    std::panic::set_hook(Box::new(|info| {
        // Log the panic to a file instead of stderr to avoid console window
        let _ = std::fs::write("retro1996_crash.log", format!("PANIC: {}", info));
        std::thread::sleep(std::time::Duration::from_secs(5));
    }));
    
    // Initialize core components
    let mut trusscore = TrussCore::new();
    let mut js_engine = ChronoScript::new();
    let mut network = match NetworkManager::new() {
        Ok(n) => n,
        Err(e) => {
            // Log error to file instead of printing to console
            let _ = std::fs::write("retro1996_error.log", format!("Failed to initialize network manager: {}", e));
            return;
        }
    };
    
    // Configure the engine for 1996-era rendering
    let mut config = EngineConfig::default();
    config.rendering_mode = RenderingMode::Netscape3;
    config.authentic_mode = true;
    config.modern_scaling = false;
    config.smoothing = false;
    config.emulate_800x600_viewport = true;
    config.enable_javascript = true;
    config.enable_images = true;
    config.enable_plugins = false;
    config.progressive_rendering = true;
    config.web_safe_palette = true;
    
    // Update the engine config
    trusscore.config = config;
    
    // Load the welcome page
    let welcome_html = match std::fs::read_to_string("assets/welcome.html") {
        Ok(html) => html,
        Err(_) => get_embedded_welcome_page()
    };
    
    trusscore.load_html(&welcome_html);
    
    // Render the page
    let display_list = trusscore.render(800.0);
    let _cmd_count = display_list.commands.len();
    
    // Test JavaScript execution
    let js_code = b"var x = 5; var y = 10; x + y;";
    let _js_result = js_engine.execute(js_code);
    
    // Test network functionality
    let test_url = "http://example.com";
    let _network_result = network.fetch(test_url);
    
    // Launch the egui window
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Retro1996 Browser")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Retro1996 Browser",
        native_options,
        Box::new(move |cc| {
            Box::new(Retro1996Browser::new(cc, trusscore, js_engine, network).unwrap())
        }),
    ).unwrap();
}

fn get_embedded_welcome_page() -> String {
    let html = "<!DOCTYPE HTML PUBLIC \"-//IETF//DTD HTML 2.0//EN\">\n";
    let html = html.to_string() + "<html>\n";
    let html = html + "<head>\n";
    let html = html + "<title>Welcome to Retro1996</title>\n";
    let html = html + "<meta http-equiv=\"Content-Type\" content=\"text/html; charset=iso-8859-1\">\n";
    let html = html + "</head>\n";
    let html = html + "<body bgcolor=\"#FFFFFF\" text=\"#000000\" link=\"#0000FF\" vlink=\"#800080\" alink=\"#FF0000\">\n";
    let html = html + "<center>\n";
    let html = html + "<h1><font color=\"#000080\" size=\"7\">Retro1996 Browser v3.0</font></h1>\n";
    let html = html + "<h2><font color=\"#000080\" size=\"5\">TrussCore Rendering Engine</font></h2>\n";
    let html = html + "<h2><font color=\"#000080\" size=\"5\">ChronoScript JavaScript Engine</font></h2>\n";
    let html = html + "<hr width=\"80%\">\n";
    let html = html + "<p><font size=\"4\">Welcome to the Retro1996 Browser!</font></p>\n";
    let html = html + "<p><font size=\"4\">This browser simulates the web browsing experience from 1996.</font></p>\n";
    let html = html + "<hr width=\"80%\">\n";
    let html = html + "<p><a href=\"http://example.com\">Visit Example.com</a></p>\n";
    let html = html + "<p><a href=\"ftp://ftp.example.com\">Browse FTP Archive</a></p>\n";
    let html = html + "<p><a href=\"gopher://gopher.example.com\">Explore Gopher</a></p>\n";
    let html = html + "<hr width=\"80%\">\n";
    let html = html + "<p><font size=\"2\">Copyright &copy; 1996 Retro1996 Project</font></p>\n";
    let html = html + "</center>\n";
    let html = html + "</body>\n";
    let html = html + "</html>\n";
    html
}