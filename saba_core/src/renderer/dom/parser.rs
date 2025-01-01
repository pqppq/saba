use crate::renderer::dom::node::{Element, ElementKind, Node, NodeKind, Window};
use crate::renderer::html::attribute::Attribute;
use crate::renderer::html::token::{HtmlToken, HtmlTokenizer};
use alloc::rc::Rc;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::str::FromStr;

type RcRefCell<T> = Rc<RefCell<T>>;

#[derive(Debug, Clone)]
pub struct HtmlParser {
    window: RcRefCell<Window>,
    mode: InsertionMode,
    original_insertion_mode: InsertionMode,
    stack_of_open_elements: Vec<RcRefCell<Node>>,
    t: HtmlTokenizer,
}

impl HtmlParser {
    pub fn new(t: HtmlTokenizer) -> Self {
        Self {
            window: Rc::new(RefCell::new(Window::new())),
            mode: InsertionMode::Initial,
            original_insertion_mode: InsertionMode::Initial,
            stack_of_open_elements: [].to_vec(),
            t,
        }
    }

    pub fn construct_tree(&mut self) -> Rc<RefCell<Window>> {
        let mut token = self.t.next();

        while token.is_some() {
            match self.mode {
                InsertionMode::Initial => {
                    // this implementation does not support DOCTYPE token
                    if let Some(HtmlToken::Char(_)) = token {
                        token = self.t.next();
                        continue;
                    }

                    self.mode = InsertionMode::BeforeHtml;
                    continue;
                }
                InsertionMode::BeforeHtml => match token {
                    // <html>
                    Some(HtmlToken::Char(c)) => {
                        if c == ' ' || c == '\n' {
                            token = self.t.next();
                            continue;
                        }
                    }
                    Some(HtmlToken::StartTag {
                        ref tag,
                        ref attributes,
                        ..
                    }) => {
                        if tag == "html" {
                            self.insert_element(tag, attributes.to_vec());
                            self.mode = InsertionMode::BeforeHead;
                            token = self.t.next();
                            continue;
                        }
                    }
                    Some(HtmlToken::EOF) | None => {
                        return self.window.clone();
                    }
                    _ => {}
                },
                InsertionMode::BeforeHead => {
                    // <head>
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => {
                            if tag == "head" {
                                self.insert_element(tag, attributes.to_vec());
                                self.mode = InsertionMode::InHead;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }
                    self.insert_element("head", Vec::new());
                    self.mode = InsertionMode::InHead;
                    continue;
                }
                InsertionMode::InHead => {
                    // </head>, <style>, </style>
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => {
                            if tag == "style" || tag == "script" {
                                self.insert_element(tag, attributes.to_vec());
                                self.original_insertion_mode = self.mode;
                                self.mode = InsertionMode::Text;
                                token = self.t.next();
                                continue;
                            }
                            if tag == "body" {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                            }
                            if let Ok(_element_kind) = ElementKind::from_str(tag) {
                                self.pop_until(ElementKind::Head);
                                self.mode = InsertionMode::AfterHead;
                                continue;
                            }
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "head" {
                                self.mode = InsertionMode::AfterHead;
                                token = self.t.next();
                                self.pop_until(ElementKind::Head);
                                continue;
                            }
                        }
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                    }
                    // ignore unsupported tag
                    token = self.t.next();
                    continue;
                }
                InsertionMode::AfterHead => {
                    // <body>
                    match token {
                        Some(HtmlToken::Char(c)) => {
                            if c == ' ' || c == '\n' {
                                self.insert_char(c);
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => {
                            if tag == "body" {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                self.mode = InsertionMode::InBody;
                                continue;
                            }
                        }
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }
                    self.insert_element("body", Vec::new());
                    self.mode = InsertionMode::InBody;
                    continue;
                }
                InsertionMode::InBody => {
                    match token {
                        Some(HtmlToken::StartTag {
                            ref tag,
                            ref attributes,
                            ..
                        }) => match tag.as_str() {
                            "p" => {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                continue;
                            }
                            "h1" | "h2" => {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                continue;
                            }
                            "a" => {
                                self.insert_element(tag, attributes.to_vec());
                                token = self.t.next();
                                continue;
                            }
                            _ => {
                                token = self.t.next();
                            }
                        },
                        Some(HtmlToken::EndTag { ref tag }) => {
                            match tag.as_str() {
                                "body" => {
                                    self.mode = InsertionMode::AfterBody;
                                    token = self.t.next();
                                    if !self.contain_in_stack(ElementKind::Body) {
                                        // parse failed. ignore token.
                                        continue;
                                    }
                                    self.pop_until(ElementKind::Body);
                                    continue;
                                }
                                "html" => {
                                    if self.pop_current_node(ElementKind::Body) {
                                        self.mode = InsertionMode::AfterBody;
                                        assert!(self.pop_current_node(ElementKind::Html));
                                    } else {
                                        token = self.t.next();
                                    }
                                    continue;
                                }
                                "p" => {
                                    token = self.t.next();
                                    self.pop_until(ElementKind::P);
                                    continue;
                                }
                                "h1" | "h2" => {
                                    let kind = ElementKind::from_str(tag)
                                        .expect("Failed to convert string to ElementKind.");
                                    token = self.t.next();
                                    self.pop_until(kind);
                                    continue;
                                }
                                "a" => {
                                    token = self.t.next();
                                    self.pop_until(ElementKind::A);
                                    continue;
                                }
                                _ => {
                                    token = self.t.next();
                                }
                            }
                        }
                        Some(HtmlToken::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                    }
                }
                InsertionMode::Text => {
                    match token {
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "style" {
                                self.pop_until(ElementKind::Style);
                                self.mode = self.original_insertion_mode;
                                token = self.t.next();
                                continue;
                            }
                            if tag == "script" {
                                self.pop_until(ElementKind::Script);
                                self.mode = self.original_insertion_mode;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::Char(c)) => {
                            self.insert_char(c);
                            token = self.t.next();
                            continue;
                        }
                        _ => {}
                    }
                    self.mode = self.original_insertion_mode;
                }
                InsertionMode::AfterBody => {
                    match token {
                        Some(HtmlToken::Char(_c)) => {
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::EndTag { ref tag }) => {
                            if tag == "html" {
                                self.mode = InsertionMode::AfterAfterBody;
                                token = self.t.next();
                                continue;
                            }
                        }
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }
                    self.mode = InsertionMode::InBody;
                }
                InsertionMode::AfterAfterBody => {
                    match token {
                        Some(HtmlToken::Char(_c)) => {
                            token = self.t.next();
                            continue;
                        }
                        Some(HtmlToken::EOF) | None => {
                            return self.window.clone();
                        }
                        _ => {}
                    }
                    self.mode = InsertionMode::InBody;
                }
            }
        }

        self.window.clone()
    }

    fn create_element(&self, tag: &str, attributes: Vec<Attribute>) -> Node {
        let elem = Element::new(tag, attributes);
        Node::new(NodeKind::Element(elem))
    }

    fn insert_element(&mut self, tag: &str, attributes: Vec<Attribute>) {
        let window = self.window.borrow();
        let current = match self.stack_of_open_elements.last() {
            Some(e) => e.clone(),
            None => window.document(),
        };

        let node = Rc::new(RefCell::new(self.create_element(tag, attributes)));

        if current.borrow().first_child().is_some() {
            let mut last_sibling = current.borrow().first_child();
            loop {
                last_sibling = match last_sibling {
                    Some(ref node) => {
                        if node.borrow().next_sibling().is_some() {
                            node.borrow().next_sibling()
                        } else {
                            break;
                        }
                    }
                    None => unimplemented!("last_sibling should be Some"),
                };
            }

            last_sibling
                .as_ref()
                .unwrap()
                .borrow_mut()
                .set_next_sibling(Some(node.clone()));
            node.borrow_mut().set_previous_sibling(Rc::downgrade(
                &last_sibling.expect("last_sibling should be Some"),
            ))
        } else {
            current.borrow_mut().set_first_child(Some(node.clone()));
        }

        current.borrow_mut().set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));

        self.stack_of_open_elements.push(node);
    }

    fn pop_current_node(&mut self, element_kind: ElementKind) -> bool {
        let current = match self.stack_of_open_elements.last() {
            Some(e) => e,
            None => return false,
        };
        if current.borrow().element_kind() == Some(element_kind) {
            // pop if last elem is target kind
            self.stack_of_open_elements.pop();
            return true;
        }
        false
    }

    fn pop_until(&mut self, element_kind: ElementKind) {
        assert!(
            self.contain_in_stack(element_kind),
            "Stack doesn't have an element {:?}",
            element_kind
        );

        loop {
            let current = match self.stack_of_open_elements.pop() {
                Some(e) => e,
                None => return,
            };
            if current.borrow().element_kind() == Some(element_kind) {
                return;
            }
        }
    }

    fn contain_in_stack(&mut self, element_kind: ElementKind) -> bool {
        self.stack_of_open_elements
            .iter()
            .map(|e| e.borrow().element_kind() == Some(element_kind))
            .any(|b| b)
    }

    fn create_char(&self, c: char) -> Node {
        let mut s = "".to_string();
        s.push(c);
        Node::new(NodeKind::Text(s))
    }

    fn insert_char(&mut self, c: char) {
        let current = match self.stack_of_open_elements.last() {
            Some(e) => e.clone(),
            None => return,
        };
        if let NodeKind::Text(ref mut s) = current.borrow_mut().kind {
            s.push(c);
            return;
        }
        if c == '\n' || c == ' ' {
            return;
        }
        let node = Rc::new(RefCell::new(self.create_char(c)));
        if current.borrow().first_child().is_some() {
            current
                .borrow()
                .first_child()
                .unwrap()
                .borrow_mut()
                .set_next_sibling(Some(node.clone()));
            node.borrow_mut().set_previous_sibling(Rc::downgrade(
                &current
                    .borrow()
                    .first_child()
                    .expect("Failed to get a first child."),
            ));
        } else {
            current.borrow_mut().set_first_child(Some(node.clone()));
        }
        current.borrow_mut().set_last_child(Rc::downgrade(&node));
        node.borrow_mut().set_parent(Rc::downgrade(&current));
        self.stack_of_open_elements.push(node);
    }
}

/// https://html.spec.whatwg.org/multipage/parsing.html#the-insertion-mode
#[derive(Debug, Clone, Copy)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    AfterHead,
    InBody,
    Text,
    AfterBody,
    AfterAfterBody,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_empty() {
        let html = "".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let expected = Rc::new(RefCell::new(Node::new(NodeKind::Document)));

        assert_eq!(expected, window.borrow().document());
    }

    #[test]
    fn test_body() {
        let html = "<html><head></head><body></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            document
        );

        let html = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "html",
                Vec::new()
            ))))),
            html
        );

        let head = html
            .borrow()
            .first_child()
            .expect("failed to get a first child of html");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "head",
                Vec::new()
            ))))),
            head
        );

        let body = head
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );
    }

    #[test]
    fn test_text() {
        let html = "<html><head></head><body>text</body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Document))),
            document
        );

        let html = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "html",
                Vec::new()
            ))))),
            html
        );

        let body = html
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );

        let text = body
            .borrow()
            .first_child()
            .expect("failed to get a first child of document");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))),
            text
        );
    }

    #[test]
    fn test_multiple_nodes() {
        let html = "<html><head></head><body><p><a foo=bar>text</a></p></body></html>".to_string();
        let t = HtmlTokenizer::new(html);
        let window = HtmlParser::new(t).construct_tree();
        let document = window.borrow().document();

        let body = document
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .first_child()
            .expect("failed to get a first child of document")
            .borrow()
            .next_sibling()
            .expect("failed to get a next sibling of head");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "body",
                Vec::new()
            ))))),
            body
        );

        let p = body
            .borrow()
            .first_child()
            .expect("failed to get a first child of body");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "p",
                Vec::new()
            ))))),
            p
        );

        let mut attr = Attribute::new();
        "foo".chars().for_each(|c| attr.add_name(c));
        "bar".chars().for_each(|c| attr.add_value(c));

        let a = p
            .borrow()
            .first_child()
            .expect("failed to get a first child of p");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Element(Element::new(
                "a",
                vec![attr]
            ))))),
            a
        );

        let text = a
            .borrow()
            .first_child()
            .expect("failed to get a first child of a");
        assert_eq!(
            Rc::new(RefCell::new(Node::new(NodeKind::Text("text".to_string())))),
            text
        );
    }
}