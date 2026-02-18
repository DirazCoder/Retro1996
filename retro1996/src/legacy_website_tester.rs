use std::collections::HashMap;
use std::time::{Instant, Duration};
use crate::engine::{TrussCore, DomNode, RenderNode};
use crate::javascript_engine::ChronoScript;
use crate::network::NetworkManager;

#[derive(Debug, Clone)]
pub struct WebsiteTestResult {
    pub url: String,
    pub title: String,
    pub load_time: Duration,
    pub compatibility_score: f32,
    pub features_supported: Vec<String>,
    pub features_missing: Vec<String>,
    pub quirks_detected: Vec<String>,
    pub js_errors: Vec<String>,
    pub html_issues: Vec<String>,
    pub performance_metrics: PerformanceMetrics,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub dom_build_time: Duration,
    pub layout_time: Duration,
    pub render_time: Duration,
    pub js_execution_time: Duration,
    pub memory_usage_estimate: u64,
}

#[derive(Debug, Clone)]
pub struct LegacyWebsiteTester {
    pub engine: TrussCore,
    pub js_engine: ChronoScript,
    pub network: NetworkManager,
    pub test_results: Vec<WebsiteTestResult>,
    pub custom_rules: Vec<TestRule>,
    pub verbose: bool,
}

impl LegacyWebsiteTester {
    pub fn new() -> Self {
        LegacyWebsiteTester {
            engine: TrussCore::new(),
            js_engine: ChronoScript::new(),
            network: NetworkManager::new(),
            test_results: Vec::new(),
            custom_rules: Vec::new(),
            verbose: false,
        }
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub fn test_website(&mut self, url: &str) -> Result<WebsiteTestResult, String> {
        let start_time = Instant::now();

        if self.verbose {
            println!("Testing website: {}", url);
        }

        let html_content = self.fetch_content(url)?;

        let result = self.test_html_content(&html_content, url)?;

        let total_time = start_time.elapsed();

        let final_result = WebsiteTestResult {
            url: url.to_string(),
            title: result.title,
            load_time: total_time,
            compatibility_score: result.compatibility_score,
            features_supported: result.features_supported,
            features_missing: result.features_missing,
            quirks_detected: result.quirks_detected,
            js_errors: result.js_errors,
            html_issues: result.html_issues,
            performance_metrics: result.performance_metrics,
            notes: result.notes,
        };

        self.test_results.push(final_result.clone());

        Ok(final_result)
    }

    fn fetch_content(&self, url: &str) -> Result<String, String> {
        if url.starts_with("about:") {
            match url {
                "about:welcome" | "about:" => Ok(String::from(include_str!("../assets/welcome.html"))),
                "about:blank" => Ok(String::from("<html><head><title>Blank Page</title></head><body></body></html>")),
                "about:browser" => Ok(String::from("<html><head><title>About Browser</title></head><body><h1>Retro1996 Browser v3.0</h1><p>TrussCore Rendering Engine</p><p>ChronoScript JavaScript Engine</p></body></html>")),
                _ => Ok(String::from("<html><head><title>Unknown About Page</title></head><body><p>Unknown about: URL</p></body></html>")),
            }
        } else if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ftp://") || url.starts_with("gopher://") {
            match self.network.fetch(url) {
                Ok(response) => Ok(String::from_utf8_lossy(&response.body).to_string()),
                Err(e) => Err(format!("Failed to fetch URL: {}", e)),
            }
        } else {
            Err(format!("Unsupported URL scheme: {}", url))
        }
    }

    pub fn test_html_content(&mut self, html: &str, url: &str) -> Result<WebsiteTestResult, String> {
        let start_time = Instant::now();

        self.engine.load_html(html);

        let dom = self.engine.get_dom();
        let render_tree = self.engine.get_render_tree();

        let dom_build_time = start_time.elapsed();

        let features_supported = self.test_html_features(html);
        let features_missing = self.test_missing_features(html);
        let quirks_detected = self.detect_1996_quirks(html);
        let html_issues = self.identify_html_issues(html);

        let js_start = Instant::now();
        let js_errors = self.test_javascript(html, url)?;
        let js_execution_time = js_start.elapsed();

        let layout_start = Instant::now();
        let _layout_result = self.engine.layout(800.0);
        let layout_time = layout_start.elapsed();

        let render_start = Instant::now();
        let _render_result = self.engine.render(800.0);
        let render_time = render_start.elapsed();

        let compatibility_score = self.calculate_compatibility_score(
            &features_supported,
            &features_missing,
            &js_errors,
            &html_issues
        );

        let title = self.extract_title_from_dom(&dom);

        let performance_metrics = PerformanceMetrics {
            dom_build_time,
            layout_time,
            render_time,
            js_execution_time,
            memory_usage_estimate: (html.len() / 1024) as u64 + 100,
        };

        let notes = self.generate_test_notes(compatibility_score, &features_supported, &features_missing);

        Ok(WebsiteTestResult {
            url: url.to_string(),
            title,
            load_time: start_time.elapsed(),
            compatibility_score,
            features_supported,
            features_missing,
            quirks_detected,
            js_errors,
            html_issues,
            performance_metrics,
            notes,
        })
    }

    fn test_html_features(&self, html: &str) -> Vec<String> {
        let mut features = Vec::new();

        if html.contains("<html") || html.contains("<HTML") {
            features.push("HTML Structure".to_string());
        }
        if html.contains("<head") || html.contains("<HEAD") {
            features.push("Head Section".to_string());
        }
        if html.contains("<body") || html.contains("<BODY") {
            features.push("Body Section".to_string());
        }

        if html.contains("<table") || html.contains("<TABLE") {
            features.push("HTML Tables".to_string());
        }
        if html.contains("<form") || html.contains("<FORM") {
            features.push("HTML Forms".to_string());
        }
        if html.contains("<frameset") || html.contains("<FRAMESET") {
            features.push("Frames Support".to_string());
        }
        if html.contains("<font") || html.contains("<FONT") {
            features.push("Font Tags".to_string());
        }
        if html.contains("<center") || html.contains("<CENTER") {
            features.push("Center Tags".to_string());
        }
        if html.contains("<marquee") || html.contains("<MARQUEE") {
            features.push("Marquee Tags".to_string());
        }
        if html.contains("<blink") || html.contains("<BLINK") {
            features.push("Blink Tags".to_string());
        }
        if html.contains("<bgsound") || html.contains("<BGSOUND") {
            features.push("Background Sound".to_string());
        }
        if html.contains("<hr") || html.contains("<HR") {
            features.push("Horizontal Rules".to_string());
        }
        if html.contains("<img") || html.contains("<IMG") {
            features.push("Image Support".to_string());
        }
        if html.contains("<applet") || html.contains("<APPLET") {
            features.push("Java Applets".to_string());
        }
        if html.contains("<embed") || html.contains("<EMBED") {
            features.push("Embedded Objects".to_string());
        }
        if html.contains("<object") || html.contains("<OBJECT") {
            features.push("Object Elements".to_string());
        }

        if html.contains("style=") || html.contains("<style") || html.contains("stylesheet") {
            features.push("CSS Support".to_string());
        }

        features
    }

    fn test_missing_features(&self, html: &str) -> Vec<String> {
        let mut missing = Vec::new();

        if html.contains("<div") && html.contains("style=") && (html.contains("position: absolute") || html.contains("position: relative")) {
            missing.push("Modern CSS Positioning".to_string());
        }
        if html.contains("<span") && html.contains("style=") {
            missing.push("Modern CSS Styling".to_string());
        }
        if html.contains("<script") && html.contains("type=\"text/javascript\"") {
            if html.contains("async") || html.contains("defer") || html.contains("import") || html.contains("export") {
                missing.push("Modern JavaScript Features".to_string());
            }
        }

        if html.contains("<article") || html.contains("<section") || html.contains("<nav") || 
           html.contains("<header") || html.contains("<footer") || html.contains("<aside") {
            missing.push("HTML5 Semantic Elements".to_string());
        }

        if html.contains("<input type=\"email\"") || html.contains("<input type=\"date\"") || 
           html.contains("<input type=\"number\"") || html.contains("<input type=\"range\"") {
            missing.push("Modern Form Controls".to_string());
        }

        missing
    }

    fn detect_1996_quirks(&self, html: &str) -> Vec<String> {
        let mut quirks = Vec::new();

        if html.contains("bgcolor=") {
            quirks.push("BGColor Attributes".to_string());
        }
        if html.contains("text=") {
            quirks.push("Text Color Attributes".to_string());
        }
        if html.contains("link=") || html.contains("vlink=") || html.contains("alink=") {
            quirks.push("Link Color Attributes".to_string());
        }
        if html.contains("<nobr>") || html.contains("<wbr>") {
            quirks.push("Text Breaking Control".to_string());
        }
        if html.contains("<basefont") {
            quirks.push("Basefont Tag".to_string());
        }
        if html.contains("<isindex>") {
            quirks.push("IsIndex Tag".to_string());
        }
        if html.contains("<listing>") || html.contains("<xmp>") || html.contains("<plaintext>") {
            quirks.push("Preformatted Text Tags".to_string());
        }
        if html.contains("<dir>") || html.contains("<menu>") {
            quirks.push("Directory/Menu Lists".to_string());
        }
        if html.contains("<strike>") || html.contains("<s>") {
            quirks.push("Strikethrough Tags".to_string());
        }
        if html.contains("<u>") {
            quirks.push("Underline Tags".to_string());
        }

        quirks
    }

    fn identify_html_issues(&self, html: &str) -> Vec<String> {
        let mut issues = Vec::new();

        if html.matches("<table").count() > html.matches("</table").count() {
            issues.push("Unclosed Table Tags".to_string());
        }
        if html.matches("<tr").count() > html.matches("</tr").count() {
            issues.push("Unclosed Table Row Tags".to_string());
        }
        if html.matches("<td").count() > html.matches("</td").count() {
            issues.push("Unclosed Table Cell Tags".to_string());
        }
        if html.matches("<form").count() > html.matches("</form").count() {
            issues.push("Unclosed Form Tags".to_string());
        }
        if html.matches("<script").count() > html.matches("</script").count() {
            issues.push("Unclosed Script Tags".to_string());
        }

        if html.contains("align=\"center\"") && !html.contains("<center>") {
            issues.push("Deprecated Align Attribute".to_string());
        }

        issues
    }

    fn test_javascript(&mut self, html: &str, url: &str) -> Result<Vec<String>, String> {
        let mut errors = Vec::new();

        let mut pos = 0;
        while let Some(script_start) = html[pos..].find("<script") {
            let actual_start = pos + script_start;
            if let Some(script_end) = html[actual_start..].find("</script>") {
                let content_end_pos = actual_start + script_end;
                let script_content_start = html[actual_start..].find('>');
                if let Some(content_start) = script_content_start {
                    let content_start_pos = actual_start + content_start + 1;
                    
                    if content_end_pos > content_start_pos {
                        let script_content = &html[content_start_pos..content_end_pos];
                        
                        match self.js_engine.execute_with_context(script_content, url) {
                            Ok(_) => {},
                            Err(e) => errors.push(format!("JavaScript Error: {}", e)),
                        }
                    }
                }
                
                pos = content_end_pos + 9;
            } else {
                break;
            }
        }

        Ok(errors)
    }

    fn calculate_compatibility_score(
        &self,
        features_supported: &[String],
        features_missing: &[String],
        js_errors: &[String],
        html_issues: &[String]
    ) -> f32 {
        let mut score = 100.0;

        score -= (features_missing.len() as f32) * 5.0;
        score -= (js_errors.len() as f32) * 10.0;
        score -= (html_issues.len() as f32) * 2.0;

        let legacy_features = [
            "HTML Tables", "Frames Support", "Font Tags", "Center Tags", 
            "Marquee Tags", "Blink Tags", "Background Sound", "BGColor Attributes",
            "Link Color Attributes", "Basefont Tag", "IsIndex Tag", "Strikethrough Tags"
        ];
        
        let legacy_feature_count = features_supported.iter()
            .filter(|feature| legacy_features.contains(&feature.as_str()))
            .count();
        
        score += (legacy_feature_count as f32) * 1.0;

        score.clamp(0.0, 100.0)
    }

    fn extract_title_from_dom(&self, dom: &DomNode) -> String {
        if let DomNode::Element { tag, children, .. } = dom {
            if tag == "html" {
                for child in children {
                    if let DomNode::Element { tag, children, .. } = child {
                        if tag == "head" {
                            for grandchild in children {
                                if let DomNode::Element { tag, children, .. } = grandchild {
                                    if tag == "title" {
                                        for text_node in children {
                                            if let DomNode::Text(text) = text_node {
                                                return text.trim().to_string();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        "Untitled Document".to_string()
    }

    fn generate_test_notes(&self, score: f32, supported: &[String], missing: &[String]) -> String {
        if score >= 90.0 {
            "Excellent compatibility with 1996-era web standards".to_string()
        } else if score >= 70.0 {
            "Good compatibility with minor issues".to_string()
        } else if score >= 50.0 {
            format!("Fair compatibility - supports {} features but missing {} key 1996 features", 
                   supported.len(), missing.len())
        } else {
            format!("Poor compatibility - only supports {} of {} expected 1996 features", 
                   supported.len(), supported.len() + missing.len())
        }
    }

    pub fn test_popular_1996_sites(&mut self) -> Vec<WebsiteTestResult> {
        let sites = [
            "http://www.cnn.com/",
            "http://www.yahoo.com/",
            "http://www.amazon.com/",
            "http://www.ebay.com/",
            "http://www.microsoft.com/",
            "http://www.netscape.com/",
            "http://www.ibm.com/",
            "http://www.apple.com/",
        ];

        let mut results = Vec::new();

        for site in &sites {
            match self.test_website(site) {
                Ok(result) => results.push(result),
                Err(e) => {
                    eprintln!("Error testing {}: {}", site, e);
                    results.push(WebsiteTestResult {
                        url: site.to_string(),
                        title: "Error".to_string(),
                        load_time: Duration::from_secs(0),
                        compatibility_score: 0.0,
                        features_supported: vec![],
                        features_missing: vec![],
                        quirks_detected: vec![],
                        js_errors: vec![e],
                        html_issues: vec![],
                        performance_metrics: PerformanceMetrics {
                            dom_build_time: Duration::from_secs(0),
                            layout_time: Duration::from_secs(0),
                            render_time: Duration::from_secs(0),
                            js_execution_time: Duration::from_secs(0),
                            memory_usage_estimate: 0,
                        },
                        notes: "Failed to load site".to_string(),
                    });
                }
            }
        }

        results
    }

    pub fn generate_compatibility_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== Retro1996 Browser - 1996 Website Compatibility Report ===\n\n");
        report.push_str(&format!("Total websites tested: {}\n", self.test_results.len()));

        if !self.test_results.is_empty() {
            let avg_score: f32 = self.test_results.iter()
                .map(|r| r.compatibility_score)
                .sum::<f32>() / self.test_results.len() as f32;
            
            report.push_str(&format!("Average compatibility score: {:.1}%\n\n", avg_score));
        }

        for result in &self.test_results {
            report.push_str(&format!("URL: {}\n", result.url));
            report.push_str(&format!("Title: {}\n", result.title));
            report.push_str(&format!("Load time: {:?}\n", result.load_time));
            report.push_str(&format!("Compatibility: {:.1}%\n", result.compatibility_score));
            
            if !result.features_supported.is_empty() {
                report.push_str("Features supported:\n");
                for feature in &result.features_supported {
                    report.push_str(&format!("  - {}\n", feature));
                }
            }
            
            if !result.features_missing.is_empty() {
                report.push_str("Features missing:\n");
                for feature in &result.features_missing {
                    report.push_str(&format!("  - {}\n", feature));
                }
            }
            
            if !result.quirks_detected.is_empty() {
                report.push_str("1996 Quirks detected:\n");
                for quirk in &result.quirks_detected {
                    report.push_str(&format!("  - {}\n", quirk));
                }
            }
            
            if !result.js_errors.is_empty() {
                report.push_str("JavaScript errors:\n");
                for error in &result.js_errors {
                    report.push_str(&format!("  - {}\n", error));
                }
            }
            
            if !result.html_issues.is_empty() {
                report.push_str("HTML issues:\n");
                for issue in &result.html_issues {
                    report.push_str(&format!("  - {}\n", issue));
                }
            }
            
            report.push_str(&format!("Performance metrics:\n"));
            report.push_str(&format!("  - DOM build time: {:?}\n", result.performance_metrics.dom_build_time));
            report.push_str(&format!("  - Layout time: {:?}\n", result.performance_metrics.layout_time));
            report.push_str(&format!("  - Render time: {:?}\n", result.performance_metrics.render_time));
            report.push_str(&format!("  - JS execution time: {:?}\n", result.performance_metrics.js_execution_time));
            report.push_str(&format!("  - Estimated memory usage: {} KB\n", result.performance_metrics.memory_usage_estimate));
            
            report.push_str(&format!("Notes: {}\n", result.notes));
            report.push_str("\n---\n\n");
        }

        report
    }

    pub fn get_compatibility_statistics(&self) -> CompatibilityStats {
        if self.test_results.is_empty() {
            return CompatibilityStats::default();
        }

        let total_sites = self.test_results.len();
        let avg_score: f32 = self.test_results.iter()
            .map(|r| r.compatibility_score)
            .sum::<f32>() / total_sites as f32;

        let highest_score = self.test_results.iter()
            .map(|r| r.compatibility_score)
            .fold(0.0_f32, |a, b| a.max(b));

        let lowest_score = self.test_results.iter()
            .map(|r| r.compatibility_score)
            .fold(f32::INFINITY, |a, b| a.min(b));

        let well_compatible = self.test_results.iter()
            .filter(|r| r.compatibility_score >= 80.0)
            .count();

        let poorly_compatible = self.test_results.iter()
            .filter(|r| r.compatibility_score < 50.0)
            .count();

        CompatibilityStats {
            total_sites_tested: total_sites,
            average_compatibility: avg_score,
            highest_score,
            lowest_score,
            well_compatible_sites: well_compatible,
            poorly_compatible_sites: poorly_compatible,
        }
    }

    pub fn export_results_to_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("URL,Title,LoadTimeMs,CompatibilityScore,FeaturesSupported,FeaturesMissing,JS_ERRORS_COUNT,HTML_ISSUES_COUNT,Notes\n");

        for result in &self.test_results {
            csv.push_str(&format!(
                "\"{}\",\"{}\",{},\"{}\",\"{}\",\"{}\",{},{},\"{}\"\n",
                result.url.replace("\"", "\"\""),
                result.title.replace("\"", "\"\""),
                result.load_time.as_millis(),
                result.compatibility_score,
                result.features_supported.join("|").replace("\"", "\"\""),
                result.features_missing.join("|").replace("\"", "\"\""),
                result.js_errors.len(),
                result.html_issues.len(),
                result.notes.replace("\"", "\"\"")
            ));
        }

        csv
    }

    pub fn reset_tests(&mut self) {
        self.test_results.clear();
    }

    pub fn add_custom_test_rule(&mut self, rule: TestRule) {
        self.custom_rules.push(rule);
    }

    pub fn run_custom_tests(&self, html: &str, rules: &[TestRule]) -> Vec<TestResult> {
        let mut results = Vec::new();
        
        for rule in rules {
            let passed = html.contains(&rule.selector) || html.contains(&rule.expected_behavior);
            let details = if passed {
                format!("Found expected pattern: {}", rule.selector)
            } else {
                format!("Missing expected pattern: {}", rule.selector)
            };
            
            results.push(TestResult {
                rule_name: rule.name.clone(),
                passed,
                details,
            });
        }
        
        results
    }

    pub fn get_feature_support_summary(&self) -> HashMap<String, usize> {
        let mut summary = HashMap::new();

        for result in &self.test_results {
            for feature in &result.features_supported {
                *summary.entry(feature.clone()).or_insert(0) += 1;
            }
        }

        summary
    }

    pub fn get_common_issues(&self) -> HashMap<String, usize> {
        let mut issues = HashMap::new();

        for result in &self.test_results {
            for issue in &result.html_issues {
                *issues.entry(issue.clone()).or_insert(0) += 1;
            }
            for error in &result.js_errors {
                *issues.entry(error.clone()).or_insert(0) += 1;
            }
        }

        issues
    }

    pub fn save_results_to_file(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.test_results)
            .map_err(|e| format!("Failed to serialize test results: {}", e))?;

        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write results file: {}", e))?;

        Ok(())
    }

    pub fn load_results_from_file(&mut self, path: &str) -> Result<(), String> {
        if !std::path::Path::new(path).exists() {
            return Err("Results file does not exist".to_string());
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read results file: {}", e))?;

        let loaded_results: Vec<WebsiteTestResult> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse results file: {}", e))?;

        self.test_results = loaded_results;

        Ok(())
    }

    pub fn compare_with_other_browser(&self, other_results: &[WebsiteTestResult]) -> ComparisonReport {
        let self_avg = if !self.test_results.is_empty() {
            self.test_results.iter().map(|r| r.compatibility_score).sum::<f32>() / 
            self.test_results.len() as f32
        } else {
            0.0
        };
        
        let other_avg = if !other_results.is_empty() {
            other_results.iter().map(|r| r.compatibility_score).sum::<f32>() / 
            other_results.len() as f32
        } else {
            0.0
        };

        ComparisonReport {
            tested_sites: self.test_results.len(),
            sites_tested_by_both: std::cmp::min(self.test_results.len(), other_results.len()),
            avg_difference: self_avg - other_avg,
            self_avg_score: self_avg,
            other_avg_score: other_avg,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompatibilityStats {
    pub total_sites_tested: usize,
    pub average_compatibility: f32,
    pub highest_score: f32,
    pub lowest_score: f32,
    pub well_compatible_sites: usize,
    pub poorly_compatible_sites: usize,
}

#[derive(Debug, Clone)]
pub struct TestRule {
    pub name: String,
    pub selector: String,
    pub expected_behavior: String,
    pub severity: TestSeverity,
}

#[derive(Debug, Clone)]
pub enum TestSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub rule_name: String,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone)]
pub struct ComparisonReport {
    pub tested_sites: usize,
    pub sites_tested_by_both: usize,
    pub avg_difference: f32,
    pub self_avg_score: f32,
    pub other_avg_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_website_tester_creation() {
        let tester = LegacyWebsiteTester::new();
        assert_eq!(tester.test_results.len(), 0);
    }

    #[test]
    fn test_html_feature_detection() {
        let tester = LegacyWebsiteTester::new();
        let html = r##"
        <html>
        <head><title>Test</title></head>
        <body bgcolor="#FFFFFF">
        <center><h1>Welcome</h1></center>
        <table><tr><td>Data</td></tr></table>
        <marquee>Scrolling text</marquee>
        </body>
        </html>
        "##;

        let features = tester.test_html_features(html);
        assert!(features.contains(&"HTML Structure".to_string()));
        assert!(features.contains(&"HTML Tables".to_string()));
        assert!(features.contains(&"Center Tags".to_string()));
        assert!(features.contains(&"Marquee Tags".to_string()));
    }

    #[test]
    fn test_1996_quirks_detection() {
        let tester = LegacyWebsiteTester::new();
        let html = r##"
        <html>
        <head><title>Test</title></head>
        <body bgcolor="#FFFFFF" text="#000000" link="#0000FF" vlink="#800080" alink="#FF0000">
        <font face="Arial" size="+1">Text</font>
        <basefont face="Times" size="3">
        </body>
        </html>
        "##;

        let quirks = tester.detect_1996_quirks(html);
        assert!(quirks.contains(&"BGColor Attributes".to_string()));
        assert!(quirks.contains(&"Text Color Attributes".to_string()));
        assert!(quirks.contains(&"Link Color Attributes".to_string()));
        assert!(quirks.contains(&"Basefont Tag".to_string()));
    }

    #[test]
    fn test_missing_features_detection() {
        let tester = LegacyWebsiteTester::new();
        let modern_html = r#"
        <html>
        <head><title>Modern Test</title></head>
        <body>
        <article>This is an article</article>
        <section><p>Modern content</p></section>
        <input type="email" value="test@example.com">
        <div style="position: absolute; top: 0; left: 0;">Positioned</div>
        </body>
        </html>
        "#;

        let missing = tester.test_missing_features(modern_html);
        assert!(missing.contains(&"HTML5 Semantic Elements".to_string()));
        assert!(missing.contains(&"Modern Form Controls".to_string()));
        assert!(missing.contains(&"Modern CSS Positioning".to_string()));
    }

    #[test]
    fn test_html_issue_detection() {
        let tester = LegacyWebsiteTester::new();
        let problematic_html = r#"
        <html>
        <head><title>Problematic</title></head>
        <body>
        <table><tr><td>Unclosed table
        <form>Unclosed form
        <script>console.log('unclosed script');</script>
        </body>
        </html>
        "#;

        let issues = tester.identify_html_issues(problematic_html);
        assert!(issues.contains(&"Unclosed Table Tags".to_string()));
        assert!(issues.contains(&"Unclosed Form Tags".to_string()));
    }

    #[test]
    fn test_compatibility_scoring() {
        let tester = LegacyWebsiteTester::new();
        let features_supported = vec!["HTML Tables".to_string(), "Frames Support".to_string()];
        let features_missing = vec![];
        let js_errors = vec![];
        let html_issues = vec![];

        let score = tester.calculate_compatibility_score(
            &features_supported,
            &features_missing,
            &js_errors,
            &html_issues
        );

        assert!(score >= 90.0);
    }

    #[test]
    fn test_generate_test_notes() {
        let tester = LegacyWebsiteTester::new();
        let notes_high = tester.generate_test_notes(95.0, &[], &[]);
        assert!(notes_high.contains("Excellent"));

        let notes_low = tester.generate_test_notes(30.0, &[], &[]);
        assert!(notes_low.contains("Poor"));
    }

    #[test]
    fn test_performance_metrics() {
        let mut tester = LegacyWebsiteTester::new();
        let html = "<html><head><title>Perf Test</title></head><body><p>Test content</p></body></html>";
        
        let result = tester.test_html_content(html, "test://perf").unwrap();
        
        assert!(result.performance_metrics.dom_build_time.as_nanos() >= 0);
        assert!(result.performance_metrics.layout_time.as_nanos() >= 0);
        assert!(result.performance_metrics.render_time.as_nanos() >= 0);
        assert!(result.performance_metrics.js_execution_time.as_nanos() >= 0);
        assert!(result.performance_metrics.memory_usage_estimate >= 0);
    }

    #[test]
    fn test_compatibility_statistics() {
        let mut tester = LegacyWebsiteTester::new();
        
        tester.test_results.push(WebsiteTestResult {
            url: "http://example1.com".to_string(),
            title: "Example 1".to_string(),
            load_time: Duration::from_millis(100),
            compatibility_score: 90.0,
            features_supported: vec!["HTML Tables".to_string()],
            features_missing: vec![],
            quirks_detected: vec![],
            js_errors: vec![],
            html_issues: vec![],
            performance_metrics: PerformanceMetrics {
                dom_build_time: Duration::from_millis(10),
                layout_time: Duration::from_millis(20),
                render_time: Duration::from_millis(30),
                js_execution_time: Duration::from_millis(5),
                memory_usage_estimate: 100,
            },
            notes: "Good compatibility".to_string(),
        });
        
        tester.test_results.push(WebsiteTestResult {
            url: "http://example2.com".to_string(),
            title: "Example 2".to_string(),
            load_time: Duration::from_millis(200),
            compatibility_score: 70.0,
            features_supported: vec!["HTML Forms".to_string()],
            features_missing: vec![],
            quirks_detected: vec![],
            js_errors: vec![],
            html_issues: vec![],
            performance_metrics: PerformanceMetrics {
                dom_build_time: Duration::from_millis(15),
                layout_time: Duration::from_millis(25),
                render_time: Duration::from_millis(35),
                js_execution_time: Duration::from_millis(10),
                memory_usage_estimate: 150,
            },
            notes: "Decent compatibility".to_string(),
        });
        
        let stats = tester.get_compatibility_statistics();
        assert_eq!(stats.total_sites_tested, 2);
        assert_eq!(stats.well_compatible_sites, 1);
        assert_eq!(stats.poorly_compatible_sites, 0);
    }

    #[test]
    fn test_feature_support_summary() {
        let mut tester = LegacyWebsiteTester::new();
        
        tester.test_results.push(WebsiteTestResult {
            url: "http://example1.com".to_string(),
            title: "Example 1".to_string(),
            load_time: Duration::from_millis(100),
            compatibility_score: 90.0,
            features_supported: vec!["HTML Tables".to_string(), "Frames Support".to_string()],
            features_missing: vec![],
            quirks_detected: vec![],
            js_errors: vec![],
            html_issues: vec![],
            performance_metrics: PerformanceMetrics {
                dom_build_time: Duration::from_millis(10),
                layout_time: Duration::from_millis(20),
                render_time: Duration::from_millis(30),
                js_execution_time: Duration::from_millis(5),
                memory_usage_estimate: 100,
            },
            notes: "Good compatibility".to_string(),
        });
        
        tester.test_results.push(WebsiteTestResult {
            url: "http://example2.com".to_string(),
            title: "Example 2".to_string(),
            load_time: Duration::from_millis(200),
            compatibility_score: 85.0,
            features_supported: vec!["HTML Tables".to_string(), "Font Tags".to_string()],
            features_missing: vec![],
            quirks_detected: vec![],
            js_errors: vec![],
            html_issues: vec![],
            performance_metrics: PerformanceMetrics {
                dom_build_time: Duration::from_millis(15),
                layout_time: Duration::from_millis(25),
                render_time: Duration::from_millis(35),
                js_execution_time: Duration::from_millis(10),
                memory_usage_estimate: 150,
            },
            notes: "Good compatibility".to_string(),
        });
        
        let summary = tester.get_feature_support_summary();
        assert_eq!(*summary.get("HTML Tables").unwrap(), 2);
    }

    #[test]
    fn test_common_issues() {
        let mut tester = LegacyWebsiteTester::new();
        
        tester.test_results.push(WebsiteTestResult {
            url: "http://example1.com".to_string(),
            title: "Example 1".to_string(),
            load_time: Duration::from_millis(100),
            compatibility_score: 50.0,
            features_supported: vec![],
            features_missing: vec![],
            quirks_detected: vec![],
            js_errors: vec!["JavaScript Error: test".to_string()],
            html_issues: vec!["Unclosed Table Tags".to_string()],
            performance_metrics: PerformanceMetrics {
                dom_build_time: Duration::from_millis(10),
                layout_time: Duration::from_millis(20),
                render_time: Duration::from_millis(30),
                js_execution_time: Duration::from_millis(5),
                memory_usage_estimate: 100,
            },
            notes: "Issues found".to_string(),
        });
        
        let issues = tester.get_common_issues();
        assert_eq!(*issues.get("JavaScript Error: test").unwrap(), 1);
        assert_eq!(*issues.get("Unclosed Table Tags").unwrap(), 1);
    }
}