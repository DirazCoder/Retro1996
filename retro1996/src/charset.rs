use encoding_rs::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CharsetDetector {
    pub detected_encoding: &'static Encoding,
    pub confidence: f32,
    pub source: DetectionSource,
}

#[derive(Debug, Clone)]
pub enum DetectionSource {
    HttpHeader,
    MetaTag,
    BOM,
    AutoDetected,
    UserPreference,
    DefaultFallback,
}

impl CharsetDetector {
    pub fn new() -> Self {
        CharsetDetector {
            detected_encoding: WINDOWS_1252,
            confidence: 0.0,
            source: DetectionSource::DefaultFallback,
        }
    }

    pub fn detect_from_html(&mut self, html_content: &str) -> &'static Encoding {
        if html_content.len() >= 3 {
            let bytes = html_content.as_bytes();
            if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                self.detected_encoding = UTF_8;
                self.confidence = 1.0;
                self.source = DetectionSource::BOM;
                return self.detected_encoding;
            }
        }

        if let Some(encoding) = self.extract_meta_charset(html_content) {
            self.detected_encoding = encoding;
            self.confidence = 0.9;
            self.source = DetectionSource::MetaTag;
            return self.detected_encoding;
        }

        if let Some(encoding) = self.auto_detect_encoding(html_content) {
            self.detected_encoding = encoding;
            self.confidence = 0.7;
            self.source = DetectionSource::AutoDetected;
            return self.detected_encoding;
        }

        self.detected_encoding = WINDOWS_1252;
        self.confidence = 0.5;
        self.source = DetectionSource::DefaultFallback;
        self.detected_encoding
    }

    pub fn extract_meta_charset(&self, html: &str) -> Option<&'static Encoding> {
        let lower_html = html.to_lowercase();
        if let Some(start) = lower_html.find("<meta") {
            let end = lower_html[start..].find('>').unwrap_or(lower_html.len() - start);
            let meta_tag = &lower_html[start..start + end];

            if meta_tag.contains("http-equiv") && meta_tag.contains("content-type") {
                if let Some(content_start) = meta_tag.find("content=") {
                    let content_part = &meta_tag[content_start + 8..];
                    let quote = content_part.chars().next().unwrap_or(' ');
                    let content_value = if quote == '"' || quote == '\'' {
                        content_part[1..].split(quote).next().unwrap_or("")
                    } else {
                        content_part.split_whitespace().next().unwrap_or("")
                    };

                    if let Some(charset_start) = content_value.find("charset=") {
                        let charset_part = &content_value[charset_start + 8..];
                        let charset = charset_part.split_whitespace().next().unwrap_or("").trim();
                        return self.encoding_from_name(charset);
                    }
                }
            }

            if let Some(charset_start) = meta_tag.find("charset=") {
                let charset_part = &meta_tag[charset_start + 8..];
                let quote = charset_part.chars().next().unwrap_or(' ');
                let charset_value = if quote == '"' || quote == '\'' {
                    charset_part[1..].split(quote).next().unwrap_or("")
                } else {
                    charset_part.split_whitespace().next().unwrap_or("")
                };

                return self.encoding_from_name(charset_value);
            }
        }
        None
    }

    pub fn auto_detect_encoding(&self, content: &str) -> Option<&'static Encoding> {
        let content_bytes = content.as_bytes();
        
        let high_bit_chars = content_bytes.iter().filter(|&&b| b > 127).count();
        let total_chars = content_bytes.len();
        
        if total_chars > 0 && (high_bit_chars as f32 / total_chars as f32) > 0.1 {
            if self.looks_like_utf8(content_bytes) {
                return Some(UTF_8);
            }
            
            return Some(WINDOWS_1252);
        }
        
        Some(WINDOWS_1252)
    }

    fn looks_like_utf8(&self, bytes: &[u8]) -> bool {
        let mut i = 0;
        while i < bytes.len() {
            let byte = bytes[i];
            
            if byte < 0x80 {
                i += 1;
            } else if byte < 0xC0 {
                return false;
            } else if byte < 0xE0 {
                if i + 1 >= bytes.len() || (bytes[i + 1] & 0xC0) != 0x80 {
                    return false;
                }
                i += 2;
            } else if byte < 0xF0 {
                if i + 2 >= bytes.len() || 
                   (bytes[i + 1] & 0xC0) != 0x80 || 
                   (bytes[i + 2] & 0xC0) != 0x80 {
                    return false;
                }
                i += 3;
            } else {
                return false;
            }
        }
        true
    }

    pub fn encoding_from_name(&self, name: &str) -> Option<&'static Encoding> {
        let normalized = name.trim().to_lowercase();
        match normalized.as_str() {
            "utf-8" | "utf8" => Some(UTF_8),
            "iso-8859-1" | "iso8859-1" | "iso88591" | "latin1" | "latin-1" => Some(WINDOWS_1252),
            "iso-8859-2" | "iso8859-2" | "iso88592" | "latin2" | "latin-2" => Some(ISO_8859_2),
            "iso-8859-5" | "iso8859-5" | "iso88595" | "cyrillic" => Some(ISO_8859_5),
            "iso-8859-7" | "iso8859-7" | "iso88597" | "greek" => Some(ISO_8859_7),
            "iso-8859-8" | "iso8859-8" | "iso88598" | "hebrew" => Some(ISO_8859_8),
            "windows-1250" | "cp1250" => Some(WINDOWS_1250),
            "windows-1251" | "cp1251" => Some(WINDOWS_1251),
            "windows-1252" | "cp1252" => Some(WINDOWS_1252),
            "windows-1253" | "cp1253" => Some(WINDOWS_1253),
            "windows-1254" | "cp1254" => Some(WINDOWS_1254),
            "windows-1255" | "cp1255" => Some(WINDOWS_1255),
            "windows-1256" | "cp1256" => Some(WINDOWS_1256),
            "koi8-r" | "koi8r" => Some(KOI8_R),
            "us-ascii" | "ascii" => Some(WINDOWS_1252),
            _ => None,
        }
    }

    pub fn convert_to_utf8(&self, bytes: &[u8]) -> (String, bool) {
        let (cow, encoding_used, had_errors) = self.detected_encoding.decode(bytes);
        (cow.into_owned(), had_errors)
    }

    pub fn convert_from_utf8(&self, text: &str) -> Vec<u8> {
        let (cow, _encoding_used, _had_errors) = self.detected_encoding.encode(text);
        cow.to_vec()
    }

    pub fn get_available_encodings() -> Vec<&'static str> {
        vec![
            "UTF-8",
            "ISO-8859-1",
            "ISO-8859-2",
            "ISO-8859-5",
            "ISO-8859-7",
            "ISO-8859-8",
            "Windows-1250",
            "Windows-1251",
            "Windows-1252",
            "Windows-1253",
            "Windows-1254",
            "Windows-1255",
            "Windows-1256",
            "KOI8-R",
            "US-ASCII",
        ]
    }
}

#[derive(Debug, Clone)]
pub struct EntityDecoder;

impl EntityDecoder {
    pub fn new() -> Self {
        EntityDecoder
    }

    pub fn decode_html_entities(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '&' {
                let entity = self.parse_entity(&mut chars);
                result.push_str(&entity);
            } else {
                result.push(ch);
            }
        }
        
        result
    }

    fn parse_entity(&self, chars: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> String {
        let mut entity = String::new();
        let mut numeric = false;
        let mut hex = false;
        
        if let Some(next) = chars.next() {
            if next == '#' {
                numeric = true;
                if let Some(hex_marker) = chars.peek() {
                    if *hex_marker == 'x' || *hex_marker == 'X' {
                        hex = true;
                        chars.next();
                    }
                }
            } else {
                entity.push(next);
            }
        } else {
            return "&".to_string();
        }
        
        while let Some(&ch) = chars.peek() {
            if ch == ';' {
                chars.next();
                break;
            }
            if !ch.is_alphanumeric() && (!numeric || !ch.is_ascii_hexdigit()) {
                break;
            }
            entity.push(ch);
            chars.next();
        }
        
        if numeric {
            if hex {
                if let Ok(code) = u32::from_str_radix(&entity, 16) {
                    if let Some(chr) = char::from_u32(code) {
                        return chr.to_string();
                    }
                }
            } else {
                if let Ok(code) = entity.parse::<u32>() {
                    if let Some(chr) = char::from_u32(code) {
                        return chr.to_string();
                    }
                }
            }
            return format!("#{}{}", if hex { "x" } else { "" }, entity);
        } else {
            match entity.as_str() {
                "lt" => "<".to_string(),
                "gt" => ">".to_string(),
                "amp" => "&".to_string(),
                "quot" => "\"".to_string(),
                "apos" => "'".to_string(),
                "nbsp" => "\u{00A0}".to_string(),
                "copy" => "\u{00A9}".to_string(),
                "reg" => "\u{00AE}".to_string(),
                "trade" => "\u{2122}".to_string(),
                "cent" => "\u{00A2}".to_string(),
                "pound" => "\u{00A3}".to_string(),
                "yen" => "\u{00A5}".to_string(),
                "euro" => "\u{20AC}".to_string(),
                "sect" => "\u{00A7}".to_string(),
                "uml" => "\u{00A8}".to_string(),
                "ordf" => "\u{00AA}".to_string(),
                "laquo" => "\u{00AB}".to_string(),
                "not" => "\u{00AC}".to_string(),
                "shy" => "\u{00AD}".to_string(),
                "deg" => "\u{00B0}".to_string(),
                "plusmn" => "\u{00B1}".to_string(),
                "sup2" => "\u{00B2}".to_string(),
                "sup3" => "\u{00B3}".to_string(),
                "acute" => "\u{00B4}".to_string(),
                "micro" => "\u{00B5}".to_string(),
                "para" => "\u{00B6}".to_string(),
                "middot" => "\u{00B7}".to_string(),
                "cedil" => "\u{00B8}".to_string(),
                "sup1" => "\u{00B9}".to_string(),
                "ordm" => "\u{00BA}".to_string(),
                "raquo" => "\u{00BB}".to_string(),
                "frac14" => "\u{00BC}".to_string(),
                "frac12" => "\u{00BD}".to_string(),
                "frac34" => "\u{00BE}".to_string(),
                "iquest" => "\u{00BF}".to_string(),
                _ => format!("&{};", entity),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CharsetConverter {
    pub from_encoding: &'static Encoding,
    pub to_encoding: &'static Encoding,
}

impl CharsetConverter {
    pub fn new(from: &'static Encoding, to: &'static Encoding) -> Self {
        CharsetConverter {
            from_encoding: from,
            to_encoding: to,
        }
    }

    pub fn convert(&self, input: &[u8]) -> Result<Vec<u8>, String> {
        let (decoded_string, _, had_errors) = self.from_encoding.decode(input);
        if had_errors {
            return Err("Decoding errors encountered".to_string());
        }

        let (encoded_bytes, _, had_errors) = self.to_encoding.encode(&decoded_string);
        if had_errors {
            return Err("Encoding errors encountered".to_string());
        }

        Ok(encoded_bytes.to_vec())
    }

    pub fn set_source_encoding(&mut self, encoding: &'static Encoding) {
        self.from_encoding = encoding;
    }

    pub fn set_target_encoding(&mut self, encoding: &'static Encoding) {
        self.to_encoding = encoding;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charset_detection() {
        let mut detector = CharsetDetector::new();
        let html = r#"<html><head><meta charset="ISO-8859-1"></head><body>Hello World</body></html>"#;
        let encoding = detector.detect_from_html(html);
        assert_eq!(encoding.name(), "windows-1252");
    }

    #[test]
    fn test_entity_decoding() {
        let decoder = EntityDecoder::new();
        let input = "Hello &amp; Welcome to &lt;Retro1996&gt;";
        let expected = "Hello & Welcome to <Retro1996>";
        let result = decoder.decode_html_entities(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_numeric_entity_decoding() {
        let decoder = EntityDecoder::new();
        let input = "&#65;&#66;&#67;";
        let expected = "ABC";
        let result = decoder.decode_html_entities(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_hex_entity_decoding() {
        let decoder = EntityDecoder::new();
        let input = "&#x41;&#x42;&#x43;";
        let expected = "ABC";
        let result = decoder.decode_html_entities(input);
        assert_eq!(result, expected);
    }
}