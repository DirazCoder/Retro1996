use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::Write;
use std::thread;
use std::time::Duration;

// Import the engine components
mod engine;
mod network;
mod ui;
mod javascript_engine;

use engine::{TrussCore, EngineConfig, RenderingMode, DisplayList};
use network::{NetworkManager, HttpResponse};
use ui::Retro1996Browser;
use javascript_engine::ChronoScript;

fn main() {
    println!("Retro1996 Browser - TrussCore Engine");
    println!("====================================");
    
    // Initialize core components
    let trusscore = TrussCore::new();
    let js_engine = ChronoScript::new();
    let network = NetworkManager::new();
    
    // Configure the engine for 1996-era rendering
    let mut config = EngineConfig::default();
    config.rendering_mode = RenderingMode::Netscape3;
    config.authentic_mode = true;
    config.modern_scaling = false;  // Disable modern scaling for authentic 1996 experience
    config.smoothing = false;       // Disable anti-aliasing
    config.emulate_800x600_viewport = true;
    config.enable_javascript = true;
    config.enable_images = true;
    config.enable_plugins = false;  // Disable plugins for now
    config.progressive_rendering = true;
    config.web_safe_palette = true;
    
    // Update the engine config
    trusscore.config = config;
    
    // Load the welcome page
    let welcome_html = match std::fs::read_to_string("assets/welcome.html") {
        Ok(html) => {
            println!("Loading welcome page from assets/welcome.html");
            html
        }
        Err(_) => {
            println!("Welcome page file not found, using embedded welcome page");
            r#"
                <!DOCTYPE HTML PUBLIC "-//IETF//DTD HTML 2.0//EN">
                <html>
                <head>
                    <title>Welcome to Retro1996</title>
                    <meta http-equiv="Content-Type" content="text/html; charset=iso-8859-1">
                </head>
                <body bgcolor="#FFFFFF" text="#000000" link="#0000FF" vlink="#800080" alink="#FF0000">
                    <center>
                        <h1><font color="#000080" size="7">Retro1996 Browser v3.0</font></h1>
                        <h2><font color="#000080" size="5">TrussCore Rendering Engine</font></h2>
                        <h2><font color="#000080" size="5">ChronoScript JavaScript Engine</font></h2>
                        <hr width="80%">
                        <p><font size="4">Welcome to the Retro1996 Browser!</font></p>
                        <p><font size="4">This browser simulates the web browsing experience from 1996.</font></p>
                        <hr width="80%">
                        <p><a href="http://example.com">Visit Example.com</a></p>
                        <p><a href="ftp://ftp.example.com">Browse FTP Archive</a></p>
                        <p><a href="gopher://gopher.example.com">Explore Gopher</a></p>
                        <hr width="80%">
                        <p><font size="2">Copyright &copy; 1996 Retro1996 Project</font></p>
                    </center>
                </body>
                </html>
            "#.to_string()
        }
    };
    
    println!("Loading welcome page...");
    trusscore.load_html(&welcome_html);
    
    // Render the page
    println!("Rendering page...");
    let display_list = trusscore.render(800.0);
    
    println!("Display list generated with {} commands", display_list.commands.len());
    
    // Test JavaScript execution
    println!("Testing JavaScript engine...");
    let js_result = js_engine.execute("var x = 5; var y = 10; x + y;");
    match js_result {
        Ok(_) => println!("JavaScript execution successful"),
        Err(e) => println!("JavaScript error: {}", e),
    }
    
    // Test network functionality
    println!("Testing network manager...");
    let test_url = "http://example.com";
    match network.fetch(test_url) {
        Ok(response) => {
            println!("Network fetch successful, got {} bytes", response.body.len());
        }
        Err(e) => {
            println!("Network error: {}", e);
        }
    }
    
    println!("Retro1996 Browser initialization complete!");
    println!("Engine is ready to render web pages from 1996.");
    
    // Keep the application running
    println!("Starting main loop...");
    let start_time = Instant::now();
    
    loop {
        thread::sleep(Duration::from_millis(1000));
        
        // Update animations if any
        trusscore.update_animated_gifs();
        
        // Update blink elements if any
        if trusscore.has_blink_elements() {
            trusscore.update_blink_state();
        }
        
        // Update marquee elements if any
        if trusscore.has_marquee_elements() {
            trusscore.update_marquee_positions(0.1);
        }
        
        // Show progress
        let elapsed = start_time.elapsed().as_secs();
        if elapsed % 10 == 0 {
            println!("Running for {} seconds...", elapsed);
        }
    }
}
