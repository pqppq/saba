use crate::renderer::css::token::CssToken;
use crate::renderer::css::token::CssTokenizer;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::iter::Peekable;

pub type ComponentValue = CssToken;

#[derive(Debug, Clone)]
pub struct CssParser {
    t: Peekable<CssTokenizer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StyleSheet {
    pub rules: Vec<QualifiedRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedRule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    TypeSelector(String),
    ClassSelector(String),
    IdSelector(String),
    UnknownSelector,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: ComponentValue,
}

impl CssParser {
    pub fn new(t: CssTokenizer) -> Self {
        Self { t: t.peekable() }
    }

    pub fn parse_stylesheet(&mut self) -> StyleSheet {
        let mut sheet = StyleSheet::new();
        sheet.set_rules(self.consume_list_of_rules());
        sheet
    }

    pub fn consume_list_of_rules(&mut self) -> Vec<QualifiedRule> {
        let mut rules = Vec::new();
        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return rules,
            };
            match token {
                CssToken::AtKeyword(_k) => {
                    // ignore unsupported media query
                    let _r = self.consume_qualified_rule();
                }
                _ => {
                    let rule = self.consume_qualified_rule();
                    match rule {
                        Some(r) => rules.push(r),
                        None => return rules,
                    }
                }
            }
        }
    }

    fn consume_qualified_rule(&mut self) -> Option<QualifiedRule> {
        let mut rule = QualifiedRule::new();
        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return None,
            };
            match token {
                CssToken::OpenCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::OpenCurly));
                    rule.set_declarations(self.consume_list_of_declarations());
                    return Some(rule);
                }
                _ => {
                    rule.set_selector(self.consume_selector());
                }
            }
        }
    }

    fn consume_selector(&mut self) -> Selector {
        let token = match self.t.next() {
            Some(t) => t,
            None => panic!("should have a token but got None"),
        };
        match token {
            CssToken::HashToken(value) => Selector::IdSelector(value[1..].to_string()),
            CssToken::Delim(delim) => {
                if delim == '.' {
                    return Selector::ClassSelector(self.consume_ident());
                }
                panic!("Parser error: {:?} is an unexpected token.", token);
            }
            CssToken::Ident(ident) => {
                // treat selecter like `a:hover` as type selector
                if self.t.peek() == Some(&CssToken::Colon) {
                    while self.t.peek() == Some(&CssToken::Colon) {
                        self.t.next();
                    }
                }
                Selector::TypeSelector(ident.to_string())
            }
            _ => {
                self.t.next();
                Selector::UnknownSelector
            }
        }
    }

    fn consume_list_of_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        loop {
            let token = match self.t.peek() {
                Some(t) => t,
                None => return declarations,
            };
            match token {
                CssToken::CloseCurly => {
                    assert_eq!(self.t.next(), Some(CssToken::CloseCurly));
                    return declarations;
                }
                CssToken::SemiColon => {
                    assert_eq!(self.t.next(), Some(CssToken::SemiColon));
                }
                CssToken::Ident(ref _i) => {
                    if let Some(declaration) = self.consume_declaration() {
                        declarations.push(declaration);
                    }
                }
                _ => {
                    self.t.next();
                }
            }
        }
    }

    fn consume_declaration(&mut self) -> Option<Declaration> {
        if self.t.peek().is_none() {
            return None;
        }
        let mut declaration = Declaration::new();
        declaration.set_property(self.consume_ident());

        match self.t.next() {
            Some(token) => match token {
                CssToken::Colon => {}
                _ => return None,
            },
            None => return None,
        }
        declaration.set_value(self.consume_component_value());
        Some(declaration)
    }

    fn consume_ident(&mut self) -> String {
        let token = match self.t.next() {
            Some(t) => t,
            None => panic!("should have a token but got None"),
        };
        match token {
            CssToken::Ident(ref ident) => ident.to_string(),
            _ => {
                panic!("Parser error: {:?} is an unexpected token.", token);
            }
        }
    }

    fn consume_component_value(&mut self) -> ComponentValue {
        self.t
            .next()
            .expect("should have a token is consume_componet_value")
    }
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: [].to_vec() }
    }

    pub fn set_rules(&mut self, rules: Vec<QualifiedRule>) {
        self.rules = rules;
    }
}

impl QualifiedRule {
    pub fn new() -> Self {
        Self {
            selector: Selector::TypeSelector("".to_string()),
            declarations: [].to_vec(),
        }
    }

    pub fn set_selector(&mut self, selector: Selector) {
        self.selector = selector;
    }

    pub fn set_declarations(&mut self, declarations: Vec<Declaration>) {
        self.declarations = declarations;
    }
}

impl Declaration {
    pub fn new() -> Self {
        Self {
            property: "".to_string(),
            value: ComponentValue::Ident("".to_string()),
        }
    }

    pub fn set_property(&mut self, property: String) {
        self.property = property;
    }

    pub fn set_value(&mut self, value: ComponentValue) {
        self.value = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_empty() {
        let style = "".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        assert_eq!(cssom.rules.len(), 0);
    }

    #[test]
    fn test_one_rule() {
        let style = "p { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::new();
        rule.set_selector(Selector::TypeSelector("p".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);
            i += 1;
        }
    }

    #[test]
    fn test_id_selector() {
        let style = "#id { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::new();
        rule.set_selector(Selector::IdSelector("id".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);
            i += 1;
        }
    }

    #[test]
    fn test_class_selector() {
        let style = ".class { color: red; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule = QualifiedRule::new();
        rule.set_selector(Selector::ClassSelector("class".to_string()));
        let mut declaration = Declaration::new();
        declaration.set_property("color".to_string());
        declaration.set_value(ComponentValue::Ident("red".to_string()));
        rule.set_declarations(vec![declaration]);

        let expected = [rule];
        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);
            i += 1;
        }
    }

    #[test]
    fn test_multiple_rules() {
        let style = "p { content: \"Hey\"; } h1 { font-size: 40; color: blue; }".to_string();
        let t = CssTokenizer::new(style);
        let cssom = CssParser::new(t).parse_stylesheet();

        let mut rule1 = QualifiedRule::new();
        rule1.set_selector(Selector::TypeSelector("p".to_string()));
        let mut declaration1 = Declaration::new();
        declaration1.set_property("content".to_string());
        declaration1.set_value(ComponentValue::StringToken("Hey".to_string()));
        rule1.set_declarations(vec![declaration1]);

        let mut rule2 = QualifiedRule::new();
        rule2.set_selector(Selector::TypeSelector("h1".to_string()));
        let mut declaration2 = Declaration::new();
        declaration2.set_property("font-size".to_string());
        declaration2.set_value(ComponentValue::Number(40.0));
        let mut declaration3 = Declaration::new();
        declaration3.set_property("color".to_string());
        declaration3.set_value(ComponentValue::Ident("blue".to_string()));
        rule2.set_declarations(vec![declaration2, declaration3]);

        let expected = [rule1, rule2];
        assert_eq!(cssom.rules.len(), expected.len());

        let mut i = 0;
        for rule in &cssom.rules {
            assert_eq!(&expected[i], rule);
            i += 1;
        }
    }
}
