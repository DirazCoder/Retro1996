# Retro1996 Browser

## Experience the Web as it Was Meant to Be in 1996

**Retro1996** is not just another web browser—it's a meticulously crafted time machine that transports you back to the golden age of the internet. Built with modern Rust for reliability and performance, this browser captures the authentic 1996 browsing experience while delivering production-grade stability.

![Retro1996 Browser Screenshot](assets/iconweb.png)

### 🚀 What Makes Retro1996 Special

In 1996, the internet was a wild frontier of discovery, creativity, and wonder. Websites were hand-crafted with care, animated GIFs danced across screens, and every new webpage felt like uncharted territory. **Retro1996** preserves this magic while making it accessible today.

### 🎯 Project Vision

**Mission**: To create the most authentic 1996-era web browsing experience possible, built with modern development practices and production-grade code quality.

**Core Principles**:
- **Authenticity First**: Every feature, UI element, and behavior captures the 1996 internet experience
- **Modern Foundation**: Built with Rust for memory safety, performance, and maintainability  
- **Production Quality**: Enterprise-grade code with comprehensive error handling, testing, and documentation
- **Educational Value**: A living museum of web history and browser development

### 🌟 Key Features

#### Core Browser Experience
- **1996-Era Rendering Engine**: Authentic HTML 3.2 and CSS Level 1 support
- **JavaScript Engine**: Custom ECMAScript 1.0 implementation for period-accurate scripting
- **Multi-Protocol Support**: HTTP/1.0, HTTPS, FTP, Gopher, and more
- **Image Formats**: Full support for GIF, JPEG, and PNG with period-appropriate rendering

#### Authentic 1996 Features
- **Animated GIF Support**: Those dancing babies and under construction signs, just like you remember
- **Basic CSS Rendering**: Period-accurate CSS Level 1 with proper limitations
- **JavaScript 1.0**: Support for early web interactivity and DHTML
- **Cookie Management**: Simple cookie storage true to 1996 implementations
- **Bookmark System**: Organize your favorite GeoCities and Angelfire sites

#### Modern Production Features
- **Memory Safety**: Rust's ownership model prevents crashes and security vulnerabilities
- **Cross-Platform**: Windows, macOS, and Linux support
- **Performance Optimized**: Modern algorithms with 1996 aesthetics
- **Comprehensive Testing**: Unit tests, integration tests, and browser compatibility testing
- **Error Handling**: Graceful degradation when encountering modern web content

### 🛠️ Technical Architecture

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
- **UI Framework**: Egui for native, responsive interfaces
- **Rendering**: Custom HTML/CSS parser and renderer
- **Networking**: TCP-based clients for multiple protocols
- **Storage**: INI files and SQLite for configuration and data
- **Audio**: Native Windows audio for 1996-era sound effects

### 🎨 1996 Aesthetic Design

Every visual element has been carefully designed to evoke the 1996 internet experience:

- **Color Schemes**: Neon blues, vibrant oranges, and that distinctive 90s aesthetic
- **Typography**: Classic web-safe fonts and period-appropriate text rendering
- **UI Elements**: Skeuomorphic buttons, gradients, and drop shadows
- **Animations**: Subtle hover effects and loading animations
- **Sound Effects**: Dial-up modem sounds, click effects, and notification chimes

### 📊 Browser Compatibility

Retro1996 is designed to handle the web as it existed in 1996:

**Perfect Support**:
- HTML 3.2 and earlier
- CSS Level 1
- JavaScript 1.0 (ECMAScript 1st Edition)
- GIF animations
- Basic form elements
- Table-based layouts

**Graceful Degradation**:
- Modern HTML/CSS is rendered with 1996 limitations
- Complex JavaScript is safely ignored or simplified
- Modern image formats fall back to supported formats

### 🚀 Getting Started

#### Prerequisites
- Rust 1.70+ 
- Windows 10/11, macOS 10.15+, or Linux with GTK

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

1. Launch Retro1996
2. Enter a URL in the address bar
3. Experience the web through 1996 eyes
4. Visit classic sites like:
   - `http://www.geocities.com/`
   - `http://www.angelfire.com/`
   - `http://www.tripod.com/`

### 🧪 Development

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

We welcome contributions that enhance the 1996 browsing experience:

1. Fork the repository
2. Create a feature branch
3. Ensure your code follows 1996 authenticity guidelines
4. Add comprehensive tests
5. Submit a pull request

**Contribution Guidelines**:
- Maintain 1996-era authenticity in all features
- Write production-grade code with proper error handling
- Include comprehensive tests for new functionality
- Document all changes thoroughly
- Respect the project's historical accuracy

### 📚 Documentation

- [API Documentation](docs/api.md)
- [Architecture Guide](docs/architecture.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Browser Compatibility](docs/compatibility.md)

### 🎭 Why 1996?

1996 was a pivotal year for the internet:
- **Netscape Navigator 3.0** dominated the browser market
- **Internet Explorer 3.0** was Microsoft's ambitious entry
- **JavaScript** was brand new and revolutionary
- **GeoCities** and **Angelfire** hosted millions of personal websites
- The web was primarily text-based with emerging multimedia
- Every website felt like a personal creation rather than corporate content

Retro1996 captures this spirit of creativity, exploration, and digital frontier spirit.

### 🔧 System Requirements

**Minimum**:
- 1 GHz processor
- 512 MB RAM
- 100 MB disk space
- Graphics card with 32 MB VRAM

**Recommended**:
- 2 GHz processor
- 2 GB RAM
- 500 MB disk space
- Graphics card with 128 MB VRAM

### 🤝 Community

Join our community of retro computing enthusiasts:

- **Discord**: [Retro1996 Community](https://discord.gg/retro1996)
- **Forum**: [Retro Computing Discussion](https://retro1996.forum)
- **GitHub**: [Issues and Discussions](https://github.com/retro1996/retro1996/issues)

### 📰 News & Updates

- **v3.0.0**: Major rewrite with production-grade architecture
- **v2.1.0**: Added HTTPS support and improved JavaScript engine
- **v2.0.0**: Complete UI overhaul with authentic 1996 aesthetics
- **v1.0.0**: Initial release with basic HTML and image rendering

### 📄 License

Retro1996 is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

### 🙏 Acknowledgments

- The pioneers of the early web
- Netscape Communications Corporation
- The W3C for establishing web standards
- Every GeoCities user who made the web personal
- The open-source community for keeping history alive

### 🎵 Soundtrack of the Internet

Put on some 1996 hits while you browse:
- "Macarena" by Los Del Rio
- "Wonderwall" by Oasis  
- "Killing Me Softly" by Fugees
- "1979" by The Smashing Pumpkins

---

**Retro1996**: Where every webpage loads like it's 1996 again.

*Built with ❤️ for the internet we remember.*