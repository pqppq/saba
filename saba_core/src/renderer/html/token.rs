use crate::renderer::html::attribute::Attribute;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlTokenizer {
    state: State,
    pos: usize,
    reconsume: bool, // only update state and reuse current char
    latest_token: Option<HtmlToken>,
    input: Vec<char>,
    buf: String,
}

impl HtmlTokenizer {
    pub fn new(html: String) -> Self {
        Self {
            state: State::Data,
            pos: 0,
            reconsume: false,
            latest_token: None,
            input: html.chars().collect(),
            buf: String::new(),
        }
    }

    fn is_eof(&self) -> bool {
        self.pos > self.input.len()
    }

    fn reconsume_input(&mut self) -> char {
        self.reconsume = false;
        self.input[self.pos - 1]
    }

    fn consume_next_input(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }

    fn create_tag(&mut self, start_tag_token: bool) {
        if start_tag_token {
            // set latest_token
            self.latest_token = Some(HtmlToken::StartTag {
                tag: "".to_string(),
                self_closing: false,
                attributes: [].to_vec(),
            });
        } else {
            // start EndTag
            self.latest_token = Some(HtmlToken::EndTag {
                tag: "".to_string(),
            });
        }
    }

    fn take_latest_token(&mut self) -> Option<HtmlToken> {
        assert!(self.latest_token.is_some());

        // as_ref: Option<T> to Option<&T>
        // cloned: Option<&T> to Option<T>
        let t = self.latest_token.as_ref().cloned();
        self.latest_token = None;
        assert!(self.latest_token.is_none());

        t
    }

    fn append_tag_name(&mut self, c: char) {
        assert!(self.latest_token.is_some());

        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag { ref mut tag, .. } | HtmlToken::EndTag { ref mut tag } => {
                    tag.push(c)
                }
                _ => panic!("`latest_token` should be either StartTag or EndTag"),
            }
        }
    }

    // add attributes to tag created by create_tag
    fn start_new_attribute(&mut self) {
        assert!(self.latest_token.is_some());

        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    ref mut attributes, ..
                } => {
                    attributes.push(Attribute::new());
                }
                _ => panic!("`latest_token` should be StartTag"),
            }
        }
    }

    fn append_attribute(&mut self, c: char, is_name: bool) {
        assert!(self.latest_token.is_some());

        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    ref mut attributes, ..
                } => {
                    let len = attributes.len();
                    assert!(len > 0);

                    match is_name {
                        true => attributes[len - 1].add_name(c),
                        false => attributes[len - 1].add_value(c),
                    }
                }
                _ => panic!("`latest_token` should be StartTag"),
            }
        }
    }

    fn set_self_closing_flag(&mut self) {
        assert!(self.latest_token.is_some());

        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    ref mut self_closing,
                    ..
                } => *self_closing = true,
                _ => panic!("`latest_token` should be StartTag"),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlToken {
    // <foo>
    StartTag {
        tag: String,
        self_closing: bool, // self closing tag or not
        attributes: Vec<Attribute>,
    },
    // <foo/>
    EndTag {
        tag: String,
    },
    // char
    Char(char),
    // End of file
    EOF,
}

impl Iterator for HtmlTokenizer {
    type Item = HtmlToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }

        loop {
            let c = match self.reconsume {
                true => self.reconsume_input(),
                false => self.consume_next_input(),
            };

            match self.state {
                State::Data => {
                    if c == '<' {
                        self.state = State::TagOpen;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    return Some(HtmlToken::Char(c));
                }
                State::TagOpen => {
                    if c == '/' {
                        // tag name ends
                        self.state = State::EndTagOpen;
                        continue;
                    }
                    if c.is_ascii_alphabetic() {
                        // tag name starts
                        self.reconsume = true;
                        self.state = State::TagName;
                        self.create_tag(true);
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    self.reconsume = true;
                    self.state = State::Data;
                }
                State::EndTagOpen => {
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    if c.is_ascii_alphabetic() {
                        // in tag name chars
                        self.reconsume = true;
                        self.state = State::TagName;
                        self.create_tag(false);
                        continue;
                    }
                }
                State::TagName => {
                    if c == ' ' {
                        // tag name ends
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '/' {
                        // self closing tag
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '>' {
                        // tag name ends
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if c.is_ascii_uppercase() {
                        // in tag name chars
                        self.append_tag_name(c.to_ascii_lowercase());
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    self.append_tag_name(c);
                }
                State::BeforeAttributeName => {
                    if c == '/' || c == '>' || self.is_eof() {
                        // no attributes
                        self.reconsume = true;
                        self.state = State::AfterAttributeName;
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::AttributeName;
                    self.start_new_attribute();
                }
                State::AttributeName => {
                    if c == ' ' || c == '/' || self.is_eof() {
                        // attribute name ends
                        self.reconsume = false;
                        self.state = State::AfterAttributeName;
                        continue;
                    }
                    if c == '=' {
                        // attribute value
                        self.state = State::BeforeAttributeValue;
                        continue;
                    }
                    // is_name is true
                    self.append_attribute(c.to_ascii_lowercase(), true);
                }
                State::AfterAttributeName => {
                    if c == ' ' {
                        // ignore white space
                        continue;
                    }
                    if c == '/' {
                        // self closing tag
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '=' {
                        // attribute value starts
                        self.state = State::BeforeAttributeValue;
                        continue;
                    }
                    if c == '>' {
                        // attributes ends
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    // next attribute starts
                    self.reconsume = true;
                    self.state = State::AttributeName;
                    self.start_new_attribute();
                }
                State::BeforeAttributeValue => {
                    if c == ' ' {
                        // ignore white space
                        continue;
                    }
                    if c == '"' {
                        self.state = State::AttributeValueDoubleQuoted;
                        continue;
                    }
                    if c == '\'' {
                        self.state = State::AttributeValueSingleQuoted;
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::AttributeValueUnquoted;
                }
                State::AttributeValueDoubleQuoted => {
                    if c == '"' {
                        // attribute value ends
                        self.state = State::AfterAttributeValueQuoted;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    // is_name false(value)
                    self.append_attribute(c, false);
                }
                State::AttributeValueSingleQuoted => {
                    if c == '\'' {
                        // attribute value ends
                        self.state = State::AfterAttributeValueQuoted;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    // is_name false (value)
                    self.append_attribute(c, false);
                }
                State::AttributeValueUnquoted => {
                    if c == ' ' {
                        // attribute value ends
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '>' {
                        // attributes end
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    self.append_attribute(c, false);
                }
                State::AfterAttributeValueQuoted => {
                    if c == ' ' {
                        // attribute value ends
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '/' {
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '>' {
                        // attributes end
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    // next attribute starts
                    self.reconsume = true;
                    self.state = State::BeforeAttributeValue;
                }
                State::SelfClosingStartTag => {
                    if c == '>' {
                        self.set_self_closing_flag();
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        // invalid parse error
                        return Some(HtmlToken::EOF);
                    }
                }
                State::ScriptData => {
                    if c == '<' {
                        self.state = State::ScriptDataLessThanSign;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::EOF);
                    }
                    return Some(HtmlToken::Char(c));
                }
                State::ScriptDataLessThanSign => {
                    if c == '/' {
                        // reset buffer
                        // is there case that '/' is divide operator or comment '//' ?
                        self.buf = "".to_string();
                        self.state = State::ScriptDataEndTagOpen;
                        continue;
                    }

                    self.reconsume = true;
                    self.state = State::ScriptData;
                    return Some(HtmlToken::Char('<'));
                }
                State::ScriptDataEndTagOpen => {
                    if c.is_ascii_alphabetic() {
                        self.reconsume = true;
                        self.state = State::ScriptDataEndTagName;
                        // start tag false
                        self.create_tag(false);
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::ScriptData;
                    return Some(HtmlToken::Char('<'));
                }
                State::ScriptDataEndTagName => {
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if c.is_ascii_alphabetic() {
                        self.buf.push(c);
                        self.append_tag_name(c.to_ascii_lowercase());
                        continue;
                    }
                    self.state = State::TemporaryBuffer;
                    self.buf = "</".to_string() + &self.buf;
                    self.buf.push(c);
                }
                State::TemporaryBuffer => {
                    self.reconsume = true;
                    if self.buf.is_empty() {
                        self.state = State::ScriptData;
                        continue;
                    }
                    // remove first char
                    let c = self
                        .buf
                        .chars()
                        .nth(0)
                        .expect("self.buf should have at least 1 chars");
                    self.buf.remove(0);
                    return Some(HtmlToken::Char(c));
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    ScriptData,             // scripts in <script>
    ScriptDataLessThanSign, // '<' sign appears in <script>
    ScriptDataEndTagOpen,
    ScriptDataEndTagName,
    TemporaryBuffer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_empty() {
        let html = "".to_string();
        let mut tokenizer = HtmlTokenizer::new(html);
        assert!(tokenizer.next().is_none());
    }

    #[test]
    fn test_start_and_end_tag() {
        let html = "<body></body>".to_string();
        let mut tokenizer = HtmlTokenizer::new(html);
        let expected = [
            HtmlToken::StartTag {
                tag: "body".to_string(),
                self_closing: false,
                attributes: Vec::new(),
            },
            HtmlToken::EndTag {
                tag: "body".to_string(),
            },
        ];
        for e in expected {
            assert_eq!(Some(e), tokenizer.next());
        }
    }

    #[test]
    fn test_attributes() {
        let html = "<p class=\"A\" id='B' foo=bar></p>".to_string();
        let mut tokenizer = HtmlTokenizer::new(html);
        let mut attr1 = Attribute::new();
        attr1.add_name('c');
        attr1.add_name('l');
        attr1.add_name('a');
        attr1.add_name('s');
        attr1.add_name('s');
        attr1.add_value('A');

        let mut attr2 = Attribute::new();
        attr2.add_name('i');
        attr2.add_name('d');
        attr2.add_value('B');

        let mut attr3 = Attribute::new();
        attr3.add_name('f');
        attr3.add_name('o');
        attr3.add_name('o');
        attr3.add_value('b');
        attr3.add_value('a');
        attr3.add_value('r');

        let expected = [
            HtmlToken::StartTag {
                tag: "p".to_string(),
                self_closing: false,
                attributes: vec![attr1, attr2, attr3],
            },
            HtmlToken::EndTag {
                tag: "p".to_string(),
            },
        ];
        for e in expected {
            assert_eq!(Some(e), tokenizer.next());
        }
    }

    #[test]
    fn test_self_closing_tag() {
        let html = "<img />".to_string();
        let mut tokenizer = HtmlTokenizer::new(html);
        let expected = [HtmlToken::StartTag {
            tag: "img".to_string(),
            self_closing: true,
            attributes: Vec::new(),
        }];
        for e in expected {
            assert_eq!(Some(e), tokenizer.next());
        }
    }

    #[test]
    fn test_script_tag() {
        let html = "<script>js code;</script>".to_string();
        let mut tokenizer = HtmlTokenizer::new(html);
        let expected = [
            HtmlToken::StartTag {
                tag: "script".to_string(),
                self_closing: false,
                attributes: Vec::new(),
            },
            HtmlToken::Char('j'),
            HtmlToken::Char('s'),
            HtmlToken::Char(' '),
            HtmlToken::Char('c'),
            HtmlToken::Char('o'),
            HtmlToken::Char('d'),
            HtmlToken::Char('e'),
            HtmlToken::Char(';'),
            HtmlToken::EndTag {
                tag: "script".to_string(),
            },
        ];
        for e in expected {
            assert_eq!(Some(e), tokenizer.next());
        }
    }
}
