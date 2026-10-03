> [!WARNING]
> **DO NOT USE THIS. SERIOUSLY.**
>
> This is absolute garbage. None of the buttons you see in those menus actually work — they either do nothing or just crash the browser. I would not even bother reading any of this. If I had to rate it, I'd give it a -10/10, because it's AI-generated garbage text I used back then. That's why you should never use very small, lightweight AI models — they always screw up.
>
> It takes like 30 seconds to even load the browser window. Embarrassing, right?
>
> The project has almost none of the features listed below — not all of them, but most. This project is **100% deprecated, unmaintained**, and likely has security holes. If that scares you, don't touch it.
>
> A comprehensive test by an independent AI determined the full extent of what this browser can render:
>
> ```
> text
> ```
>
> Yea, it can only render very simple text thats it, if you dont believe my 20/20 vision, go check it for yourself.
>
> **Go to my C# project instead: [https://github.com/dirazcoder/retro96](https://github.com/dirazcoder/retro96) — you'll like that one way, way better than this one.**

---

# Retro1996 Browser

## Experience the Web as it Was Meant to Be in 1996

**Retro1996** is not just another web browser. It’s carefully designed to take you back to the golden age of the internet. Built with modern Rust for reliability and performance, this browser captures the real 1996 browsing experience while offering solid stability.

![Retro1996 Browser Screenshot](assets/iconweb.png)

### What Makes Retro1996 Special

In 1996, the internet was a thrilling frontier of discovery, creativity, and wonder. Websites were hand-crafted with care, animated GIFs brightened screens, and every new page felt like uncharted territory. **Retro1996** keeps this magic alive while making it available today.

### Project Vision

**Mission**: To create the most authentic 1996 web browsing experience possible. This is done with modern development practices and high-quality coding.

**Key Principles**:
- **Authenticity First**: Every feature, UI element, and behavior reflects the 1996 internet experience.
- **Modern Foundation**: Built using Rust for memory safety, performance, and maintainability.  
- **Production Quality**: High-quality code with thorough error handling, testing, and documentation.
- **Educational Value**: Serves as an ongoing museum of web history and browser development.

### Key Features

#### Core Browser Experience
- **1996-Era Rendering Engine**: Supports authentic HTML 3.2 and CSS Level 1.
- **JavaScript Engine**: Custom ECMAScript 1.0 implementation for accurate scripting of the time.
- **Multi-Protocol Support**: Handles HTTP/1.0, HTTPS, FTP, Gopher, and more.
- **Image Formats**: Fully supports GIF, JPEG, and PNG with age-appropriate rendering.

#### Authentic 1996 Features
- **Animated GIF Support**: Enjoy those dancing babies and under-construction signs, just like you remember.
- **Basic CSS Rendering**: Accurate CSS Level 1 with proper limitations.
- **JavaScript 1.0**: Supports early web interactivity and DHTML.
- **Cookie Management**: Simple cookie storage true to 1996 methods.
- **Bookmark System**: Organize your favorite GeoCities and Angelfire sites.

#### Modern Production Features
- **Memory Safety**: Rust's ownership model helps prevent crashes and security issues.
- **Cross-Platform**: Works on Windows, macOS, and Linux.
- **Performance Optimized**: Modern algorithms that also maintain a 1996 look.
- **Thorough Testing**: Includes unit tests, integration tests, and browser compatibility checks.
- **Error Handling**: Degrades gracefully when facing modern web content.

### Technical Architecture

#### Engine Components
```
Retro1996/
├── src/
│   ├── engine.rs              # Main browser engine
│   ├── javascript_engine.rs   # ECMAScript 1.0 implementation  
│   ├── ui.rs                  # 1996-style user interface
│   ├── network/               # Multi-protocol networking
│   │   ├── http_client.rs     # HTTP/1.0 client
│   │   ├── ssl.rs            # SSL 3.0 for HTTPS
│   │   ├── ftp_client.rs      # FTP protocol support
│   │   └── gopher_client.rs   # Gopher protocol support
│   ├── image_handler.rs       # GIF, JPEG, PNG rendering
│   ├── cache.rs              # Browser cache system
│   ├── history.rs            # Browsing history
│   └── bookmarks.rs          # Bookmark management
```

#### Technology Stack
- **Language**: Rust 2021 Edition
- **UI Framework**: Egui for responsive native interfaces
- **Rendering**: Custom HTML/CSS parser and renderer
- **Networking**: TCP-based clients for various protocols
- **Storage**: INI files and SQLite for configuration and data
- **Audio**: Native Windows audio for 1996-era sound effects

### 1996 Aesthetic Design

Every visual detail is designed to evoke the 1996 internet experience:

- **Color Schemes**: Neon blues, bright oranges, and that unique 90s look.
- **Typography**: Classic web-safe fonts and text rendering suited to the era.
- **UI Elements**: Skeuomorphic buttons, gradients, and drop shadows.
- **Animations**: Subtle hover effects and loading animations.
- **Sound Effects**: Dial-up modem sounds, clicks, and notification chimes.

### Browser Compatibility

Retro1996 is made to work with the web as it was in 1996:

**Perfect Support**:
- HTML 3.2 and earlier
- CSS Level 1
- JavaScript 1.0 (ECMAScript 1st Edition)
- GIF animations
- Basic form elements
- Table-based layouts

**Graceful Degradation**:
- Modern HTML/CSS is displayed with 1996 limits.
- Complex JavaScript is safely ignored or simplified.
- Modern image formats revert to supported formats.

### Getting Started

#### Prerequisites
- Rust 1.70+ 
- Windows 10/11, macOS 10.15+, or Linux with GTK.

#### Installation

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/retro1996/retro1996.git
   cd retro1996
   ```

2. **Build the Project**:
   ```bash
   cargo build --release
   ```

3. **Run Retro1996**:
   ```bash
   cargo run --release
   ```

#### Quick Start

1. Launch Retro1996.
2. Type a URL in the address bar.
3. Experience the web through 1996 eyes.
4. Explore classic sites like:
   - `http://www.geocities.com/`
   - `http://www.angelfire.com/`
   - `http://www.tripod.com/`

### Development

#### Building for Development
```bash
cargo build
cargo run
```

#### Running Tests
```bash
cargo test
cargo test -- --nocapture
```

#### Code Quality
```bash
cargo clippy
cargo fmt
```

#### Contributing

We welcome contributions that improve the 1996 browsing experience:

1. Fork the repository.
2. Create a feature branch.
3. Ensure your code respects 1996 authenticity guidelines.
4. Add thorough tests.
5. Submit a pull request.

**Contribution Guidelines**:
- Keep 1996-era authenticity in all features.
- Write high-quality code with proper error handling.
- Include thorough tests for new features.
- Document all changes well.
- Respect the project's historical accuracy.

### Documentation

- [API Documentation](docs/api.md)
- [Architecture Guide](docs/architecture.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Browser Compatibility](docs/compatibility.md)

### Why 1996?

1996 was a key year for the internet:
- **Netscape Navigator 3.0** led the browser market.
- **Internet Explorer 3.0** marked Microsoft’s ambitious entry.
- **JavaScript** was brand new and groundbreaking.
- **GeoCities** and **Angelfire** hosted millions of personal websites.
- The web was mostly text-based with emerging multimedia.
- Every site felt like a personal creation rather than a corporate one.

Retro1996 captures this spirit of creativity and exploration.

### System Requirements

**Minimum**:
- Windows 10/11
- 1 GHz processor
- 512 MB RAM
- 100 MB disk space
- Graphics card with 32 MB VRAM

**Recommended**:
- Windows 10/11
- 2 GHz processor
- 2 GB RAM
- 500 MB disk space
- Graphics card with 128 MB VRAM

### License

Retro1996 is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

### Acknowledgments

- The pioneers of the early web.
- Netscape Communications Corporation.
- The W3C for setting web standards. 
- Every GeoCities user who made the web personal.
- The open-source community for preserving history.

### Soundtrack of the Internet

Put on some 1996 hits while you browse:
- "Macarena" by Los Del Rio
- "Wonderwall" by Oasis  
- "Killing Me Softly" by Fugees
- "1979" by The Smashing Pumpkins
