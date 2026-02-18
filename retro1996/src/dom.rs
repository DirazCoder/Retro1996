use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::str::Chars;

use crate::engine::{DomNode, StyleContext, BoxModel, SpecialElement, EngineError, QuirkProfile, WebSafePalette};

#[derive(Debug, Clone)]
pub struct DomTree {
    pub root: DomNode,
    pub node_count: usize,
    pub depth: usize,
    pub element_count: usize,
    pub text_count: usize,
    pub comment_count: usize,
    pub last_modified: Instant,
    pub node_id_counter: usize,
    pub id_map: HashMap<String, usize>,
    pub class_map: HashMap<String, Vec<usize>>,
    pub tag_map: HashMap<String, Vec<usize>>,
}

impl DomTree {
    pub fn new() -> Self {
        DomTree {
            root: DomNode::Element {
                tag: "root".to_string(),
                attributes: HashMap::new(),
                children: Vec::new(),
                id: None,
                class: None,
                node_id: 0,
            },
            node_count: 1,
            depth: 0,
            element_count: 1,
            text_count: 0,
            comment_count: 0,
            last_modified: Instant::now(),
            node_id_counter: 1,
            id_map: HashMap::new(),
            class_map: HashMap::new(),
            tag_map: HashMap::new(),
        }
    }

    pub fn from_node(root: DomNode) -> Self {
        let mut tree = DomTree::new();
        tree.root = root;
        tree.update_statistics();
        tree.build_index();
        tree
    }

    pub fn get_root(&self) -> &DomNode {
        &self.root
    }

    pub fn get_root_mut(&mut self) -> &mut DomNode {
        &mut self.root
    }

    pub fn find_by_id(&self, id: &str) -> Option<&DomNode> {
        if let Some(&node_id) = self.id_map.get(id) {
            self.find_by_node_id(node_id)
        } else {
            None
        }
    }

    pub fn find_by_id_mut(&mut self, id: &str) -> Option<&mut DomNode> {
        if let Some(&node_id) = self.id_map.get(id) {
            self.find_by_node_id_mut(node_id)
        } else {
            None
        }
    }

    pub fn find_by_class(&self, class: &str) -> Vec<&DomNode> {
        if let Some(node_ids) = self.class_map.get(class) {
            node_ids.iter().filter_map(|&id| self.find_by_node_id(id)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&DomNode> {
        if let Some(node_ids) = self.tag_map.get(tag) {
            node_ids.iter().filter_map(|&id| self.find_by_node_id(id)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_by_node_id(&self, node_id: usize) -> Option<&DomNode> {
        self.traverse_find_by_node_id(&self.root, node_id)
    }

    pub fn find_by_node_id_mut(&mut self, node_id: usize) -> Option<&mut DomNode> {
        Self::traverse_find_by_node_id_mut(&mut self.root, node_id)
    }

    fn traverse_find_by_node_id<'a>(&self, node: &'a DomNode, target_id: usize) -> Option<&'a DomNode> {
        match node {
            DomNode::Element { node_id, .. } if *node_id == target_id => Some(node),
            DomNode::Element { children, .. } => {
                for child in children {
                    if let Some(found) = self.traverse_find_by_node_id(child, target_id) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
        }
    }

    fn traverse_find_by_node_id_mut(node: &mut DomNode, target_id: usize) -> Option<&mut DomNode> {
        match node {
            DomNode::Element { node_id, .. } if *node_id == target_id => Some(node),
            DomNode::Element { children, .. } => {
                for child in children {
                    if let Some(found) = Self::traverse_find_by_node_id_mut(child, target_id) {
                        return Some(found);
                    }
                }
                None
            },
            _ => None,
        }
    }

    pub fn insert_child(&mut self, parent_id: usize, mut child: DomNode) -> Result<(), EngineError> {
        let child_id = self.node_id_counter;
        self.node_id_counter += 1;
        
        if let DomNode::Element { node_id: ref mut id, .. } = &mut child {
            *id = child_id;
        }
        
        let inserted = Self::insert_child_recursive(&mut self.root, parent_id, child);
        if inserted {
            self.update_statistics();
            self.build_index();
            self.last_modified = Instant::now();
            Ok(())
        } else {
            Err(EngineError::NotFound(format!("Parent node with ID {} not found", parent_id)))
        }
    }
    
    fn insert_child_recursive(node: &mut DomNode, parent_id: usize, child: DomNode) -> bool {
        match node {
            DomNode::Element { node_id, children, .. } => {
                if *node_id == parent_id {
                    children.push(child);
                    return true;
                }
                for c in children.iter_mut() {
                    if Self::insert_child_recursive(c, parent_id, child.clone()) {
                        return true;
                    }
                }
                false
            },
            _ => false,
        }
    }

    pub fn remove_child(&mut self, parent_id: usize, child_id: usize) -> Result<(), EngineError> {
        if let Some(parent) = self.find_by_node_id_mut(parent_id) {
            if let DomNode::Element { children, .. } = parent {
                let initial_len = children.len();
                children.retain(|child| {
                    if let DomNode::Element { node_id, .. } = child {
                        *node_id != child_id
                    } else {
                        true
                    }
                });
                
                if children.len() < initial_len {
                    self.update_statistics();
                    self.build_index();
                    self.last_modified = Instant::now();
                    Ok(())
                } else {
                    Err(EngineError::NotFound(format!("Child node with ID {} not found", child_id)))
                }
            } else {
                Err(EngineError::InvalidOperation("Cannot remove child from non-element node".to_string()))
            }
        } else {
            Err(EngineError::NotFound(format!("Parent node with ID {} not found", parent_id)))
        }
    }

    pub fn set_attribute(&mut self, node_id: usize, key: &str, value: &str) -> Result<(), EngineError> {
        if let Some(node) = self.find_by_node_id_mut(node_id) {
            if let DomNode::Element { attributes, .. } = node {
                attributes.insert(key.to_string(), value.to_string());
                self.last_modified = Instant::now();
                Ok(())
            } else {
                Err(EngineError::InvalidOperation("Cannot set attribute on non-element node".to_string()))
            }
        } else {
            Err(EngineError::NotFound(format!("Node with ID {} not found", node_id)))
        }
    }

    pub fn get_attribute(&self, node_id: usize, key: &str) -> Option<String> {
        if let Some(node) = self.find_by_node_id(node_id) {
            if let DomNode::Element { attributes, .. } = node {
                attributes.get(key).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn remove_attribute(&mut self, node_id: usize, key: &str) -> Result<(), EngineError> {
        if let Some(node) = self.find_by_node_id_mut(node_id) {
            if let DomNode::Element { attributes, .. } = node {
                attributes.remove(key);
                self.last_modified = Instant::now();
                Ok(())
            } else {
                Err(EngineError::InvalidOperation("Cannot remove attribute from non-element node".to_string()))
            }
        } else {
            Err(EngineError::NotFound(format!("Node with ID {} not found", node_id)))
        }
    }

    pub fn set_text_content(&mut self, node_id: usize, text: &str) -> Result<(), EngineError> {
        if let Some(node) = self.find_by_node_id_mut(node_id) {
            match node {
                DomNode::Element { children, .. } => {
                    *children = vec![DomNode::Text(text.to_string())];
                    self.last_modified = Instant::now();
                    Ok(())
                },
                DomNode::Text(ref mut content) => {
                    *content = text.to_string();
                    self.last_modified = Instant::now();
                    Ok(())
                },
                _ => Err(EngineError::InvalidOperation("Cannot set text content on this node type".to_string())),
            }
        } else {
            Err(EngineError::NotFound(format!("Node with ID {} not found", node_id)))
        }
    }

    pub fn get_text_content(&self, node_id: usize) -> Option<String> {
        if let Some(node) = self.find_by_node_id(node_id) {
            self.collect_text_content(node)
        } else {
            None
        }
    }

    fn collect_text_content(&self, node: &DomNode) -> Option<String> {
        match node {
            DomNode::Text(text) => Some(text.clone()),
            DomNode::Element { children, .. } => {
                let mut content = String::new();
                for child in children {
                    if let Some(child_text) = self.collect_text_content(child) {
                        content.push_str(&child_text);
                    }
                }
                if content.is_empty() {
                    None
                } else {
                    Some(content)
                }
            },
            _ => None,
        }
    }

    pub fn query_selector(&self, selector: &str) -> Option<&DomNode> {
        self.query_selector_recursive(&self.root, selector)
    }

    pub fn query_selector_all(&self, selector: &str) -> Vec<&DomNode> {
        let mut results = Vec::new();
        self.query_selector_all_recursive(&self.root, selector, &mut results);
        results
    }

    fn query_selector_recursive<'a>(&self, node: &'a DomNode, selector: &str) -> Option<&'a DomNode> {
        if self.matches_selector(node, selector) {
            return Some(node);
        }

        if let DomNode::Element { children, .. } = node {
            for child in children {
                if let Some(matched) = self.query_selector_recursive(child, selector) {
                    return Some(matched);
                }
            }
        }

        None
    }

    fn query_selector_all_recursive<'a>(&self, node: &'a DomNode, selector: &str, results: &mut Vec<&'a DomNode>) {
        if self.matches_selector(node, selector) {
            results.push(node);
        }

        if let DomNode::Element { children, .. } = node {
            for child in children {
                self.query_selector_all_recursive(child, selector, results);
            }
        }
    }

    fn matches_selector(&self, node: &DomNode, selector: &str) -> bool {
        match node {
            DomNode::Element { tag, id, class, .. } => {
                let parts: Vec<&str> = selector.split('.').collect();
                let tag_part = parts[0];
                
                if !tag_part.is_empty() && tag != tag_part {
                    return false;
                }
                
                if parts.len() > 1 {
                    if let Some(class_name) = class {
                        let class_parts: Vec<&str> = parts[1..].iter().map(|s| s.split('#').next().unwrap_or("")).collect();
                        for class_part in class_parts {
                            if !class_name.contains(class_part) {
                                return false;
                            }
                        }
                    } else {
                        return false;
                    }
                }
                
                if let Some(id_part) = selector.split('#').nth(1) {
                    if let Some(node_id) = id {
                        if node_id != id_part {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                
                true
            },
            _ => false,
        }
    }

    pub fn traverse(&self, callback: &mut dyn FnMut(&DomNode)) {
        self.traverse_recursive(&self.root, callback);
    }

    fn traverse_recursive(&self, node: &DomNode, callback: &mut dyn FnMut(&DomNode)) {
        callback(node);
        
        if let DomNode::Element { children, .. } = node {
            for child in children {
                self.traverse_recursive(child, callback);
            }
        }
    }

    pub fn get_statistics(&self) -> DomStatistics {
        DomStatistics {
            node_count: self.node_count,
            depth: self.depth,
            element_count: self.element_count,
            text_count: self.text_count,
            comment_count: self.comment_count,
            last_modified: self.last_modified.elapsed().as_millis() as u64,
        }
    }

    fn update_statistics(&mut self) {
        self.node_count = 0;
        self.depth = 0;
        self.element_count = 0;
        self.text_count = 0;
        self.comment_count = 0;
        
        let root = self.root.clone();
        self.calculate_statistics(&root, 0);
    }

    fn calculate_statistics(&mut self, node: &DomNode, depth: usize) {
        self.node_count += 1;
        self.depth = self.depth.max(depth);
        
        match node {
            DomNode::Element { children, .. } => {
                self.element_count += 1;
                for child in children {
                    self.calculate_statistics(child, depth + 1);
                }
            },
            DomNode::Text(_) => {
                self.text_count += 1;
            },
            DomNode::Comment(_) => {
                self.comment_count += 1;
            },
        }
    }

    fn build_index(&mut self) {
        self.id_map.clear();
        self.class_map.clear();
        self.tag_map.clear();
        
        let root = self.root.clone();
        self.index_node(&root);
    }

    fn index_node(&mut self, node: &DomNode) {
        match node {
            DomNode::Element { id, class, tag, node_id, children, .. } => {
                if let Some(id_value) = id {
                    self.id_map.insert(id_value.clone(), *node_id);
                }
                
                if let Some(class_value) = class {
                    self.class_map.entry(class_value.clone())
                        .or_insert_with(Vec::new)
                        .push(*node_id);
                }
                
                self.tag_map.entry(tag.clone())
                    .or_insert_with(Vec::new)
                    .push(*node_id);
                
                for child in children {
                    self.index_node(child);
                }
            },
            _ => {},
        }
    }

    fn generate_node_id(&mut self) -> usize {
        let id = self.node_id_counter;
        self.node_id_counter += 1;
        id
    }

    fn assign_node_id(&mut self, node: &mut DomNode, node_id: usize) {
        match node {
            DomNode::Element { node_id: ref mut id, .. } => {
                *id = node_id;
            },
            _ => {},
        }
    }

    pub fn clone_subtree(&self, node_id: usize) -> Option<DomNode> {
        self.find_by_node_id(node_id).cloned()
    }

    pub fn replace_node(&mut self, old_id: usize, new_node: DomNode) -> Result<(), EngineError> {
        if Self::replace_node_recursive(&mut self.root, old_id, new_node) {
            self.update_statistics();
            self.build_index();
            self.last_modified = Instant::now();
            Ok(())
        } else {
            Err(EngineError::NotFound(format!("Node with ID {} not found", old_id)))
        }
    }
    
    fn replace_node_recursive(node: &mut DomNode, old_id: usize, new_node: DomNode) -> bool {
        match node {
            DomNode::Element { children, .. } => {
                for child in children.iter_mut() {
                    if let DomNode::Element { node_id, .. } = child {
                        if *node_id == old_id {
                            *child = new_node;
                            return true;
                        }
                    }
                    if Self::replace_node_recursive(child, old_id, new_node.clone()) {
                        return true;
                    }
                }
                false
            },
            _ => false,
        }
    }

    fn find_parent_of_node(&self, node_id: usize) -> Option<&DomNode> {
        self.find_parent_recursive(&self.root, node_id)
    }

    fn find_parent_recursive<'a>(&self, node: &'a DomNode, target_id: usize) -> Option<&'a DomNode> {
        match node {
            DomNode::Element { children, .. } => {
                for child in children {
                    if let DomNode::Element { node_id, .. } = child {
                        if *node_id == target_id {
                            return Some(node);
                        }
                    }
                    
                    if let Some(parent) = self.find_parent_recursive(child, target_id) {
                        return Some(parent);
                    }
                }
                None
            },
            _ => None,
        }
    }

    pub fn get_children(&self, node_id: usize) -> Vec<&DomNode> {
        if let Some(node) = self.find_by_node_id(node_id) {
            if let DomNode::Element { children, .. } = node {
                children.iter().collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn get_parent(&self, node_id: usize) -> Option<&DomNode> {
        self.find_parent_of_node(node_id)
    }

    pub fn get_siblings(&self, node_id: usize) -> Vec<&DomNode> {
        if let Some(parent) = self.get_parent(node_id) {
            if let DomNode::Element { children, .. } = parent {
                children.iter().filter(|child| {
                    if let DomNode::Element { node_id: child_id, .. } = child {
                        *child_id != node_id
                    } else {
                        true
                    }
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.node_count == 0
    }

    pub fn clear(&mut self) {
        self.root = DomNode::Element {
            tag: "root".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: 0,
        };
        self.node_count = 1;
        self.depth = 0;
        self.element_count = 1;
        self.text_count = 0;
        self.comment_count = 0;
        self.last_modified = Instant::now();
        self.node_id_counter = 1;
        self.id_map.clear();
        self.class_map.clear();
        self.tag_map.clear();
    }

    pub fn validate_structure(&self) -> Result<(), EngineError> {
        self.validate_node(&self.root, 0)
    }

    fn validate_node(&self, node: &DomNode, depth: usize) -> Result<(), EngineError> {
        if depth > 1000 {
            return Err(EngineError::ParseError("Maximum DOM depth exceeded".to_string()));
        }

        match node {
            DomNode::Element { tag, attributes: _, children, id, class, node_id: _ } => {
                if tag.is_empty() {
                    return Err(EngineError::ParseError("Element tag cannot be empty".to_string()));
                }

                if let Some(id_value) = id {
                    if id_value.is_empty() {
                        return Err(EngineError::ParseError("Element ID cannot be empty".to_string()));
                    }
                }

                if let Some(class_value) = class {
                    if class_value.is_empty() {
                        return Err(EngineError::ParseError("Element class cannot be empty".to_string()));
                    }
                }

                for child in children {
                    self.validate_node(child, depth + 1)?;
                }
            },
            DomNode::Text(text) => {
                if text.is_empty() {
                    return Err(EngineError::ParseError("Text node cannot be empty".to_string()));
                }
            },
            DomNode::Comment(comment) => {
                if comment.is_empty() {
                    return Err(EngineError::ParseError("Comment node cannot be empty".to_string()));
                }
            },
        }

        Ok(())
    }

    pub fn optimize(&mut self) {
        self.remove_empty_text_nodes();
        self.merge_adjacent_text_nodes();
        self.update_statistics();
        self.build_index();
    }

    fn remove_empty_text_nodes(&mut self) {
        if let DomNode::Element { children, .. } = &mut self.root {
            children.retain(|child| {
                if let DomNode::Text(text) = child {
                    !text.trim().is_empty()
                } else {
                    true
                }
            });
        }
    }

    fn merge_adjacent_text_nodes(&mut self) {
        if let DomNode::Element { children, .. } = &mut self.root {
            let mut i = 0;
            while i < children.len() {
                if let DomNode::Text(_) = &children[i] {
                    let mut merged_text = String::new();
                    let mut j = i;
                    
                    while j < children.len() {
                        if let DomNode::Text(text) = &children[j] {
                            merged_text.push_str(text);
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    
                    if j > i + 1 {
                        children.drain(i..j);
                        children.insert(i, DomNode::Text(merged_text));
                    }
                    
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
    }

    pub fn serialize(&self) -> String {
        self.serialize_node(&self.root, 0)
    }

    fn serialize_node(&self, node: &DomNode, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        
        match node {
            DomNode::Element { tag, attributes, children, id, class, node_id: _ } => {
                let mut result = format!("{}<{}", indent_str, tag);
                
                if let Some(id_value) = id {
                    result.push_str(&format!(" id=\"{}\"", id_value));
                }
                
                if let Some(class_value) = class {
                    result.push_str(&format!(" class=\"{}\"", class_value));
                }
                
                for (key, value) in attributes {
                    result.push_str(&format!(" {}=\"{}\"", key, value));
                }
                
                if children.is_empty() {
                    result.push_str(" />\n");
                } else {
                    result.push_str(">\n");
                    for child in children {
                        result.push_str(&self.serialize_node(child, indent + 1));
                    }
                    result.push_str(&format!("{}</{}>\n", indent_str, tag));
                }
                
                result
            },
            DomNode::Text(text) => {
                format!("{}{}\n", indent_str, text)
            },
            DomNode::Comment(comment) => {
                format!("{}<!-- {} -->\n", indent_str, comment)
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct DomStatistics {
    pub node_count: usize,
    pub depth: usize,
    pub element_count: usize,
    pub text_count: usize,
    pub comment_count: usize,
    pub last_modified: u64,
}

#[derive(Debug, Clone)]
pub struct DomValidator {
    pub max_depth: usize,
    pub allow_empty_text: bool,
    pub allow_empty_elements: bool,
    pub validate_ids: bool,
    pub validate_classes: bool,
}

impl DomValidator {
    pub fn new() -> Self {
        DomValidator {
            max_depth: 1000,
            allow_empty_text: false,
            allow_empty_elements: true,
            validate_ids: true,
            validate_classes: true,
        }
    }

    pub fn validate(&self, tree: &DomTree) -> Result<(), EngineError> {
        if tree.depth > self.max_depth {
            return Err(EngineError::ParseError(format!("DOM depth {} exceeds maximum {}", tree.depth, self.max_depth)));
        }

        if !self.allow_empty_text && tree.text_count == 0 {
            return Err(EngineError::ParseError("DOM contains no text nodes".to_string()));
        }

        if !self.allow_empty_elements && tree.element_count == 0 {
            return Err(EngineError::ParseError("DOM contains no element nodes".to_string()));
        }

        Ok(())
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DomValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dom_tree() {
        let tree = DomTree::new();
        assert_eq!(tree.node_count, 1);
        assert_eq!(tree.element_count, 1);
        assert!(tree.is_empty() == false);
    }

    #[test]
    fn test_find_by_id() {
        let mut tree = DomTree::new();
        let child = DomNode::Element {
            tag: "div".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: Some("test-id".to_string()),
            class: None,
            node_id: 1,
        };
        
        tree.insert_child(0, child).unwrap();
        tree.build_index();
        
        let found = tree.find_by_id("test-id");
        assert!(found.is_some());
        assert_eq!(found.unwrap().tag(), "div");
    }

    #[test]
    fn test_query_selector() {
        let mut tree = DomTree::new();
        let child = DomNode::Element {
            tag: "div".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: Some("test-id".to_string()),
            class: Some("test-class".to_string()),
            node_id: 1,
        };
        
        tree.insert_child(0, child).unwrap();
        tree.build_index();
        
        let found = tree.query_selector("div.test-class");
        assert!(found.is_some());
        assert_eq!(found.unwrap().tag(), "div");
    }

    #[test]
    fn test_set_attribute() {
        let mut tree = DomTree::new();
        let child = DomNode::Element {
            tag: "div".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: 1,
        };
        
        tree.insert_child(0, child).unwrap();
        
        tree.set_attribute(1, "class", "test-class").unwrap();
        let class = tree.get_attribute(1, "class");
        assert_eq!(class, Some("test-class".to_string()));
    }

    #[test]
    fn test_remove_child() {
        let mut tree = DomTree::new();
        let child = DomNode::Element {
            tag: "div".to_string(),
            attributes: HashMap::new(),
            children: Vec::new(),
            id: None,
            class: None,
            node_id: 1,
        };
        
        tree.insert_child(0, child).unwrap();
        assert_eq!(tree.get_children(0).len(), 1);
        
        tree.remove_child(0, 1).unwrap();
        assert_eq!(tree.get_children(0).len(), 0);
    }

    #[test]
    fn test_validate_structure() {
        let tree = DomTree::new();
        let result = tree.validate_structure();
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize() {
        let tree = DomTree::new();
        let serialized = tree.serialize();
        assert!(serialized.contains("<root>"));
    }
}