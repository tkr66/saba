use alloc::string::String;
use alloc::vec::Vec;

use super::attribute::Attribute;

pub struct HtmlTokenizer {
    state: State,
    pos: usize,
    reconsume: bool,
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

    /// Creates a start tag if `start_tag_token` is `true`, otherwise create an end tag,
    /// Both with empty names and no attributes.
    fn create_tag(&mut self, start_tag_token: bool) {
        if start_tag_token {
            self.latest_token = Some(HtmlToken::StartTag {
                tag: String::new(),
                self_closing: false,
                attributes: Vec::new(),
            })
        } else {
            self.latest_token = Some(HtmlToken::EndTag { tag: String::new() })
        }
    }

    fn reconsume_input(&mut self) -> char {
        self.reconsume = false;
        self.input[self.pos - 1]
    }

    fn take_latest_token(&mut self) -> Option<HtmlToken> {
        assert!(self.latest_token.is_some());
        self.latest_token.take()
    }

    fn append_tag_name(&mut self, c: char) {
        assert!(self.latest_token.is_some());

        if let Some(t) = self.latest_token.as_mut() {
            match t {
                HtmlToken::StartTag {
                    tag,
                    self_closing: _,
                    attributes: _,
                } => tag.push(c),
                HtmlToken::EndTag { tag } => tag.push(c),
                _ => panic!("`latest_token` should be either StartTag or EndTag"),
            }
        }
    }

    /// Creates a new attribute with empty strings in the latest token.
    fn start_new_attribute(&mut self) {
        assert!(self.latest_token.is_some());
        match self.latest_token.as_mut().unwrap() {
            HtmlToken::StartTag {
                tag: _,
                self_closing: _,
                attributes,
            } => {
                attributes.push(Attribute::default());
            }
            _ => panic!("`latest_token` should be a StartTag"),
        }
    }

    fn append_attribute(&mut self, c: char, is_name: bool) {
        assert!(self.latest_token.is_some());
        match self.latest_token.as_mut().unwrap() {
            HtmlToken::StartTag {
                tag: _,
                self_closing: _,
                attributes,
            } => match attributes.last_mut() {
                Some(attr) => attr.add_char(c, is_name),
                None => panic!("attribute must be exists"),
            },
            _ => panic!("`latest_token` should be a StartTag"),
        }
    }

    fn set_self_closing_flag(&mut self) {
        assert!(self.latest_token.is_some());
        match self.latest_token.as_mut().unwrap() {
            HtmlToken::StartTag {
                tag: _,
                ref mut self_closing,
                attributes: _,
            } => *self_closing = true,
            _ => panic!("`latest_token` should be a StartTag"),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum HtmlToken {
    StartTag {
        tag: String,
        self_closing: bool,
        attributes: Vec<Attribute>,
    },

    EndTag {
        tag: String,
    },

    Char(char),

    Eof,
}

pub enum State {
    /// https://html.spec.whatwg.org/multipage/parsing.html#data-state
    Data,
    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-open-state
    TagOpen,
    /// https://html.spec.whatwg.org/multipage/parsing.html#end-tag-open-state
    EndTagOpen,
    /// https://html.spec.whatwg.org/multipage/parsing.html#tag-name-state
    TagName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-name-state
    BeforeAttributeName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-name-state
    AttributeName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-name-state
    AfterAttributeName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#before-attribute-value-state
    BeforeAttributeValue,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(double-quoted)-state
    AttributeValueDoubleQuoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(single-quoted)-state
    AttributeValueSingleQuoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#attribute-value-(unquoted)-state
    AttributeValueUnquoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#after-attribute-value-(quoted)-state
    AfterAttributeValueQuoted,
    /// https://html.spec.whatwg.org/multipage/parsing.html#self-closing-start-tag-state
    SelfClosingStartTag,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-state
    ScriptData,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-less-than-sign-state
    ScriptDataLessThanSign,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-end-tag-open-state
    ScriptDataEndTagOpen,
    /// https://html.spec.whatwg.org/multipage/parsing.html#script-data-end-tag-name-state
    ScriptDataEndTagName,
    /// https://html.spec.whatwg.org/multipage/parsing.html#temporary-buffer
    TemporaryBuffer,
}

impl HtmlTokenizer {
    fn consume_next_input(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }
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
                        return Some(HtmlToken::Eof);
                    }
                    return Some(HtmlToken::Char(c));
                }
                State::TagOpen => {
                    if c == '/' {
                        self.state = State::EndTagOpen;
                        continue;
                    }
                    if c.is_ascii_alphabetic() {
                        self.reconsume = true;
                        self.state = State::TagName;
                        self.create_tag(true);
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    self.reconsume = true;
                    self.state = State::Data;
                }
                State::EndTagOpen => {
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    if c.is_ascii_alphabetic() {
                        self.reconsume = true;
                        self.state = State::TagName;
                        self.create_tag(false);
                        continue;
                    }
                }
                State::TagName => {
                    if c.is_whitespace() {
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '/' {
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if c.is_ascii_uppercase() {
                        self.append_tag_name(c.to_ascii_lowercase());
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    self.append_tag_name(c);
                }
                State::BeforeAttributeName => {
                    if c == '/' || c == '>' || self.is_eof() {
                        self.reconsume = true;
                        self.state = State::AfterAttributeName;
                        continue;
                    }
                    self.reconsume = true;
                    self.state = State::AttributeName;
                    self.start_new_attribute();
                }
                State::AttributeName => {
                    if c.is_whitespace() || c == '/' || c == '>' || self.is_eof() {
                        self.reconsume = true;
                        self.state = State::AfterAttributeName;
                        continue;
                    }
                    if c == '=' {
                        self.state = State::BeforeAttributeValue;
                        continue;
                    }
                    if c.is_ascii_uppercase() {
                        self.append_attribute(c.to_ascii_lowercase(), true);
                        continue;
                    }
                    self.append_attribute(c, true);
                }
                State::AfterAttributeName => {
                    if c.is_whitespace() {
                        continue;
                    }
                    if c == '/' {
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '=' {
                        self.state = State::BeforeAttributeValue;
                        continue;
                    }
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    self.reconsume = true;
                    self.state = State::AttributeName;
                    self.start_new_attribute();
                }
                State::BeforeAttributeValue => {
                    if c.is_whitespace() {
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
                    // missing-attribute-value parse error
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    self.reconsume = true;
                    self.state = State::AttributeValueUnquoted;
                }
                State::AttributeValueDoubleQuoted => {
                    if c == '"' {
                        self.state = State::AfterAttributeValueQuoted;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    self.append_attribute(c, false);
                }
                State::AttributeValueSingleQuoted => {
                    if c == '\'' {
                        self.state = State::AfterAttributeValueQuoted;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    self.append_attribute(c, false);
                }
                State::AttributeValueUnquoted => {
                    if c.is_whitespace() {
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    // unexpected-character-in-unquoted-attribute-value parse error
                    // Includes code points that the parser encounters
                    // such as U+0022 ("), U+0027 ('), U+003C (<), U+003D (=), or U+0060 (`)
                    self.append_attribute(c, false);
                }
                State::AfterAttributeValueQuoted => {
                    if c.is_whitespace() {
                        self.state = State::BeforeAttributeName;
                        continue;
                    }
                    if c == '/' {
                        self.state = State::SelfClosingStartTag;
                        continue;
                    }
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    // missing-whitespace-between-attributes parse error
                    // Treats as if ASCII whitespace is present
                    self.reconsume = true;
                    self.state = State::BeforeAttributeName;
                }
                State::SelfClosingStartTag => {
                    if c == '>' {
                        self.set_self_closing_flag();
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    // eof-in-tag parse error
                    // The tag will be ignored
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    // unexpected-solidus-in-tag parse error
                    // Treats as if it encountered ASCII whitespace
                    self.reconsume = true;
                    self.state = State::BeforeAttributeName;
                }
                State::ScriptData => {
                    if c == '<' {
                        self.state = State::ScriptDataLessThanSign;
                        continue;
                    }
                    if self.is_eof() {
                        return Some(HtmlToken::Eof);
                    }
                    return Some(HtmlToken::Char(c));
                }
                State::ScriptDataLessThanSign => {
                    if c == '/' {
                        self.buf = String::new();
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
                        self.create_tag(false);
                    }
                    self.reconsume = true;
                    self.state = State::ScriptData;
                    // The specification returns two tokens: '<' and '/'
                    // However, here we can only return one token
                    return Some(HtmlToken::Char('<'));
                }
                State::ScriptDataEndTagName => {
                    if c == '>' {
                        self.state = State::Data;
                        return self.take_latest_token();
                    }
                    if c.is_ascii_alphabetic() {
                        self.append_tag_name(c.to_ascii_lowercase());
                        self.buf.push(c);
                        continue;
                    }
                    self.state = State::TemporaryBuffer;
                    self.buf = String::from("</") + &self.buf;
                    self.buf.push(c);
                    continue;
                }
                State::TemporaryBuffer => {
                    self.reconsume = true;

                    if self.buf.is_empty() {
                        self.state = State::Data;
                        continue;
                    }

                    let c = self
                        .buf
                        .chars()
                        .nth(0)
                        .expect("self.buf should have at least 1 char");
                    self.buf.remove(0);
                    return Some(HtmlToken::Char(c));
                }
            }
        }
    }
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
        attr1.add_char('c', true);
        attr1.add_char('l', true);
        attr1.add_char('a', true);
        attr1.add_char('s', true);
        attr1.add_char('s', true);
        attr1.add_char('A', false);

        let mut attr2 = Attribute::new();
        attr2.add_char('i', true);
        attr2.add_char('d', true);
        attr2.add_char('B', false);

        let mut attr3 = Attribute::new();
        attr3.add_char('f', true);
        attr3.add_char('o', true);
        attr3.add_char('o', true);
        attr3.add_char('b', false);
        attr3.add_char('a', false);
        attr3.add_char('r', false);

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
