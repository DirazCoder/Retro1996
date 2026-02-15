use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FormElementType {
    Text,
    Password,
    Checkbox,
    Radio,
    Submit,
    Reset,
    File,
    Hidden,
    Image,
    Button,
    Select,
    Textarea,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormElement {
    pub name: String,
    pub element_type: FormElementType,
    pub value: String,
    pub default_value: String,
    pub required: bool,
    pub readonly: bool,
    pub disabled: bool,
    pub checked: bool,  // For checkboxes and radio buttons
    pub maxlength: Option<usize>,
    pub size: Option<u32>,
    pub tabindex: Option<i32>,
    pub accesskey: Option<char>,
    pub placeholder: Option<String>,
    pub options: Vec<SelectOption>, // For select elements
    pub rows: Option<u32>, // For textarea
    pub cols: Option<u32>, // For textarea
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub text: String,
    pub selected: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form {
    pub name: String,
    pub action: String,
    pub method: HttpMethod,
    pub target: String,
    pub enctype: String,  // application/x-www-form-urlencoded, multipart/form-data, text/plain
    pub elements: Vec<FormElement>,
    pub id: Option<String>,
    pub class: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
}

impl Form {
    pub fn new(name: String, action: String, method: HttpMethod) -> Self {
        Form {
            name,
            action,
            method,
            target: "_self".to_string(),
            enctype: "application/x-www-form-urlencoded".to_string(),
            elements: Vec::new(),
            id: None,
            class: None,
            style: None,
        }
    }

    pub fn add_element(&mut self, element: FormElement) {
        self.elements.push(element);
    }

    pub fn get_element_by_name(&self, name: &str) -> Option<&FormElement> {
        self.elements.iter().find(|element| element.name == name)
    }

    pub fn get_element_by_name_mut(&mut self, name: &str) -> Option<&mut FormElement> {
        self.elements.iter_mut().find(|element| element.name == name)
    }

    pub fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        
        for element in &self.elements {
            if element.required && element.value.is_empty() && element.element_type != FormElementType::Hidden {
                errors.push(ValidationError {
                    field_name: element.name.clone(),
                    error_type: ValidationErrorType::RequiredField,
                    message: format!("Field '{}' is required", element.name),
                });
            }
            
            if let Some(maxlength) = element.maxlength {
                if element.value.len() > maxlength {
                    errors.push(ValidationError {
                        field_name: element.name.clone(),
                        error_type: ValidationErrorType::MaxLengthExceeded,
                        message: format!("Field '{}' exceeds maximum length of {}", element.name, maxlength),
                    });
                }
            }
            
            // Additional validation rules can be added here
        }
        
        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
        }
    }

    pub fn submit(&self) -> Result<FormData, ValidationResult> {
        let validation_result = self.validate();
        
        if !validation_result.is_valid {
            return Err(validation_result);
        }
        
        let mut form_data = FormData::new();
        
        for element in &self.elements {
            if element.disabled {
                continue;
            }
            
            match element.element_type {
                FormElementType::Checkbox | FormElementType::Radio => {
                    if element.checked {
                        form_data.values.insert(element.name.clone(), element.value.clone());
                    }
                },
                FormElementType::Select => {
                    // Find the selected option
                    for option in &element.options {
                        if option.selected {
                            form_data.values.insert(element.name.clone(), option.value.clone());
                            break;
                        }
                    }
                },
                FormElementType::File => {
                    // Files are handled specially - maybe store file path or content
                    if !element.value.is_empty() {
                        form_data.values.insert(element.name.clone(), element.value.clone());
                    }
                },
                _ => {
                    if !element.value.is_empty() || element.element_type == FormElementType::Hidden {
                        form_data.values.insert(element.name.clone(), element.value.clone());
                    }
                }
            }
        }
        
        Ok(form_data)
    }

    pub fn reset(&mut self) {
        for element in &mut self.elements {
            element.value = element.default_value.clone();
            
            match element.element_type {
                FormElementType::Checkbox | FormElementType::Radio => {
                    element.checked = false;
                },
                FormElementType::Select => {
                    // Reset to default selected option
                    for option in &mut element.options {
                        option.selected = option.value == element.default_value;
                    }
                },
                _ => {}
            }
        }
    }

    pub fn set_value(&mut self, field_name: &str, value: String) -> bool {
        if let Some(element) = self.get_element_by_name_mut(field_name) {
            element.value = value;
            true
        } else {
            false
        }
    }

    pub fn check_radio_group(&mut self, group_name: &str, value: &str) {
        for element in &mut self.elements {
            if element.name == group_name && element.element_type == FormElementType::Radio {
                element.checked = element.value == value;
            }
        }
    }

    pub fn toggle_checkbox(&mut self, field_name: &str) -> bool {
        if let Some(element) = self.get_element_by_name_mut(field_name) {
            if element.element_type == FormElementType::Checkbox {
                element.checked = !element.checked;
                element.value = if element.checked { "on".to_string() } else { "off".to_string() };
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormData {
    pub values: HashMap<String, String>,
}

impl FormData {
    pub fn new() -> Self {
        FormData {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn encode_url_encoded(&self) -> String {
        let pairs: Vec<String> = self.values.iter()
            .map(|(key, value)| {
                format!("{}={}", url_encode(key), url_encode(value))
            })
            .collect();
        pairs.join("&")
    }

    pub fn encode_multipart(&self, boundary: &str) -> String {
        let mut result = String::new();
        
        for (key, value) in &self.values {
            result.push_str(&format!("--{}\r\n", boundary));
            result.push_str(&format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", key));
            result.push_str(value);
            result.push_str("\r\n");
        }
        
        result.push_str(&format!("--{}--\r\n", boundary));
        result
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug)]
pub struct ValidationError {
    pub field_name: String,
    pub error_type: ValidationErrorType,
    pub message: String,
}

#[derive(Debug)]
pub enum ValidationErrorType {
    RequiredField,
    MaxLengthExceeded,
    InvalidFormat,
    CustomValidation,
}

impl FormElement {
    pub fn new(name: String, element_type: FormElementType) -> Self {
        FormElement {
            name,
            element_type,
            value: String::new(),
            default_value: String::new(),
            required: false,
            readonly: false,
            disabled: false,
            checked: false,
            maxlength: None,
            size: None,
            tabindex: None,
            accesskey: None,
            placeholder: None,
            options: Vec::new(),
            rows: None,
            cols: None,
        }
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
        if self.element_type == FormElementType::Checkbox || self.element_type == FormElementType::Radio {
            self.checked = !self.value.is_empty();
        }
    }

    pub fn set_required(&mut self, required: bool) {
        self.required = required;
    }

    pub fn set_maxlength(&mut self, maxlength: usize) {
        self.maxlength = Some(maxlength);
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        if self.element_type == FormElementType::Checkbox || self.element_type == FormElementType::Radio {
            self.value = if checked { "on".to_string() } else { "off".to_string() };
        }
    }
}

// Utility functions for form processing
pub fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

pub fn url_decode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let next1 = chars.next();
            let next2 = chars.next();
            
            if let (Some(c1), Some(c2)) = (next1, next2) {
                if let Ok(hex_byte) = u8::from_str_radix(&format!("{}{}", c1, c2), 16) {
                    if let Some(decoded_char) = char::from_u32(hex_byte as u32) {
                        result.push(decoded_char);
                    } else {
                        result.push('%');
                        result.push(c1);
                        result.push(c2);
                    }
                } else {
                    result.push('%');
                    result.push(c1);
                    result.push(c2);
                }
            } else {
                result.push('%');
                if let Some(c1) = next1 { result.push(c1); }
                if let Some(c2) = next2 { result.push(c2); }
            }
        } else {
            result.push(c);
        }
    }
    
    result
}

// Form processor for handling form submissions
pub struct FormProcessor;

impl FormProcessor {
    pub fn process_get_request(form: &Form, params: &HashMap<String, String>) -> Result<FormData, String> {
        let mut form_data = FormData::new();
        
        for element in &form.elements {
            if let Some(value) = params.get(&element.name) {
                form_data.values.insert(element.name.clone(), value.clone());
            }
        }
        
        Ok(form_data)
    }

    pub fn process_post_request(form: &Form, body: &str, content_type: &str) -> Result<FormData, String> {
        if content_type.contains("application/x-www-form-urlencoded") {
            let mut form_data = FormData::new();
            
            for pair in body.split('&') {
                let parts: Vec<&str> = pair.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = url_decode(parts[0]);
                    let value = url_decode(parts[1]);
                    form_data.values.insert(key, value);
                }
            }
            
            Ok(form_data)
        } else if content_type.contains("multipart/form-data") {
            // Simplified multipart processing
            // In a real implementation, this would be more complex
            Err("Multipart form processing not fully implemented".to_string())
        } else {
            Err(format!("Unsupported content type: {}", content_type))
        }
    }
}

// Form validator with customizable rules
pub struct FormValidator;

impl FormValidator {
    pub fn add_custom_validation<F>(field_name: &str, validator: F) -> Box<dyn Fn(&str) -> Result<(), String>>
    where
        F: Fn(&str) -> Result<(), String> + 'static,
    {
        Box::new(validator)
    }

    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.contains('@') && email.contains('.') && email.find('@') < email.rfind('.') {
            Ok(())
        } else {
            Err("Invalid email format".to_string())
        }
    }

    pub fn validate_length_range(value: &str, min: usize, max: usize) -> Result<(), String> {
        if value.len() < min {
            Err(format!("Value must be at least {} characters", min))
        } else if value.len() > max {
            Err(format!("Value must be at most {} characters", max))
        } else {
            Ok(())
        }
    }
}