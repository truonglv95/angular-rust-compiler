//! Digest Module
//!
//! Corresponds to packages/compiler/src/i18n/digest.ts
//! Computes message IDs using various hashing algorithms

use crate::i18n::i18n_ast::{self as i18n, Message, Node, Visitor};
use std::collections::HashMap;

/// Return the message id or compute it using the XLIFF1 digest.
pub fn digest(message: &Message) -> String {
    if !message.id.is_empty() {
        message.id.clone()
    } else {
        compute_digest(message)
    }
}

/// Compute the message id using the XLIFF1 digest.
pub fn compute_digest(message: &Message) -> String {
    let serialized_nodes = serialize_nodes(&message.nodes);
    let content = format!("{}[{}]", serialized_nodes.join(""), message.meaning);
    sha1(&content)
}

/// Return the message id or compute it using the XLIFF2/XMB/$localize digest.
pub fn decimal_digest(message: &Message) -> String {
    if !message.id.is_empty() {
        message.id.clone()
    } else {
        compute_decimal_digest(message)
    }
}

/// Compute the message id using the XLIFF2/XMB/$localize digest.
pub fn compute_decimal_digest(message: &Message) -> String {
    let mut visitor = SerializerIgnoreIcuExpVisitor;
    let parts: Vec<String> = message
        .nodes
        .iter()
        .map(|node| {
            let result = node.visit(&mut visitor, None);
            *result.downcast::<String>().unwrap()
        })
        .collect();
    compute_msg_id(&parts.join(""), &message.meaning)
}

/// Serialize the i18n ast to something xml-like in order to generate an UID.
pub fn serialize_nodes(nodes: &[Node]) -> Vec<String> {
    let mut visitor = SerializerVisitor;
    nodes
        .iter()
        .map(|node| {
            let result = node.visit(&mut visitor, None);
            *result.downcast::<String>().unwrap()
        })
        .collect()
}

/// Serializer visitor for generating UIDs
struct SerializerVisitor;

impl Visitor for SerializerVisitor {
    fn visit_text(&mut self, text: &i18n::Text, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(text.value.clone())
    }

    fn visit_container(&mut self, container: &i18n::Container, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children: Vec<String> = container
            .children
            .iter()
            .map(|child| {
                let result = child.visit(self, None);
                *result.downcast::<String>().unwrap()
            })
            .collect();
        Box::new(format!("[{}]", children.join(", ")))
    }

    fn visit_icu(&mut self, icu: &i18n::Icu, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let mut str_cases = Vec::new();
        for (k, v) in &icu.cases {
            let case_result = v.visit(self, None);
            let case_str = *case_result.downcast::<String>().unwrap();
            str_cases.push(format!("{} {{{}}}", k, case_str));
        }
        Box::new(format!(
            "{{{}, {}, {}}}",
            icu.expression,
            icu.type_,
            str_cases.join(", ")
        ))
    }

    fn visit_tag_placeholder(&mut self, ph: &i18n::TagPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        if ph.is_void {
            Box::new(format!("<ph tag name=\"{}\"/>", ph.start_name))
        } else {
            let children: Vec<String> = ph
                .children
                .iter()
                .map(|child| {
                    let result = child.visit(self, None);
                    *result.downcast::<String>().unwrap()
                })
                .collect();
            Box::new(format!(
                "<ph tag name=\"{}\">{}</ph name=\"{}\">",
                ph.start_name,
                children.join(", "),
                ph.close_name
            ))
        }
    }

    fn visit_placeholder(&mut self, ph: &i18n::Placeholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        if !ph.value.is_empty() {
            Box::new(format!("<ph name=\"{}\">{}</ph>", ph.name, ph.value))
        } else {
            Box::new(format!("<ph name=\"{}\"/>", ph.name))
        }
    }

    fn visit_icu_placeholder(&mut self, ph: &i18n::IcuPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let icu_result = i18n::Node::Icu(ph.value.clone()).visit(self, None);
        let icu_str = *icu_result.downcast::<String>().unwrap();
        Box::new(format!("<ph icu name=\"{}\">{}</ph>", ph.name, icu_str))
    }

    fn visit_block_placeholder(&mut self, ph: &i18n::BlockPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children: Vec<String> = ph
            .children
            .iter()
            .map(|child| {
                let result = child.visit(self, None);
                *result.downcast::<String>().unwrap()
            })
            .collect();
        Box::new(format!(
            "<ph block name=\"{}\">{}</ph name=\"{}\">",
            ph.start_name,
            children.join(", "),
            ph.close_name
        ))
    }
}

/// Serializer visitor that ignores ICU expressions
struct SerializerIgnoreIcuExpVisitor;

impl Visitor for SerializerIgnoreIcuExpVisitor {
    fn visit_text(&mut self, text: &i18n::Text, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(text.value.clone())
    }

    fn visit_container(&mut self, container: &i18n::Container, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children: Vec<String> = container
            .children
            .iter()
            .map(|child| {
                let result = child.visit(self, None);
                *result.downcast::<String>().unwrap()
            })
            .collect();
        Box::new(format!("[{}]", children.join(", ")))
    }

    fn visit_icu(&mut self, icu: &i18n::Icu, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let mut str_cases = Vec::new();
        for (k, v) in &icu.cases {
            let case_result = v.visit(self, None);
            let case_str = *case_result.downcast::<String>().unwrap();
            str_cases.push(format!("{} {{{}}}", k, case_str));
        }
        // Do not take the expression into account
        Box::new(format!("{{{}, {}}}", icu.type_, str_cases.join(", ")))
    }

    fn visit_tag_placeholder(&mut self, ph: &i18n::TagPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        if ph.is_void {
            Box::new(format!("<ph tag name=\"{}\"/>", ph.start_name))
        } else {
            let children: Vec<String> = ph
                .children
                .iter()
                .map(|child| {
                    let result = child.visit(self, None);
                    *result.downcast::<String>().unwrap()
                })
                .collect();
            Box::new(format!(
                "<ph tag name=\"{}\">{}</ph name=\"{}\">",
                ph.start_name,
                children.join(", "),
                ph.close_name
            ))
        }
    }

    fn visit_placeholder(&mut self, ph: &i18n::Placeholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        if !ph.value.is_empty() {
            Box::new(format!("<ph name=\"{}\">{}</ph>", ph.name, ph.value))
        } else {
            Box::new(format!("<ph name=\"{}\"/>", ph.name))
        }
    }

    fn visit_icu_placeholder(&mut self, ph: &i18n::IcuPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let icu_result = i18n::Node::Icu(ph.value.clone()).visit(self, None);
        let icu_str = *icu_result.downcast::<String>().unwrap();
        Box::new(format!("<ph icu name=\"{}\">{}</ph>", ph.name, icu_str))
    }

    fn visit_block_placeholder(&mut self, ph: &i18n::BlockPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        let children: Vec<String> = ph
            .children
            .iter()
            .map(|child| {
                let result = child.visit(self, None);
                *result.downcast::<String>().unwrap()
            })
            .collect();
        Box::new(format!(
            "<ph block name=\"{}\">{}</ph name=\"{}\">",
            ph.start_name,
            children.join(", "),
            ph.close_name
        ))
    }
}

/// Compute the SHA1 of the given string
///
/// WARNING: this function has not been designed nor tested with security in mind.
/// DO NOT USE IT IN A SECURITY SENSITIVE CONTEXT.
pub fn sha1(s: &str) -> String {
    let utf8 = s.as_bytes();
    let words32 = bytes_to_words32(utf8, Endian::Big);
    let len = utf8.len() * 8;

    let mut w = vec![0u32; 80];
    let mut a = 0x67452301u32;
    let mut b = 0xefcdab89u32;
    let mut c = 0x98badcfeu32;
    let mut d = 0x10325476u32;
    let mut e = 0xc3d2e1f0u32;

    let mut words32_mut = words32;
    words32_mut[len >> 5] |= 0x80 << (24 - (len % 32));
    let idx = (((len + 64) >> 9) << 4) + 15;
    if idx >= words32_mut.len() {
        words32_mut.resize(idx + 1, 0);
    }
    words32_mut[idx] = len as u32;

    for i in (0..words32_mut.len()).step_by(16) {
        let h0 = a;
        let h1 = b;
        let h2 = c;
        let h3 = d;
        let h4 = e;

        for j in 0..80 {
            if j < 16 {
                w[j] = words32_mut[i + j];
            } else {
                w[j] = rol32(w[j - 3] ^ w[j - 8] ^ w[j - 14] ^ w[j - 16], 1);
            }

            let (f, k) = fk(j, b, c, d);
            let temp = add32(add32(add32(add32(rol32(a, 5), f), e), k), w[j]);
            e = d;
            d = c;
            c = rol32(b, 30);
            b = a;
            a = temp;
        }

        a = add32(a, h0);
        b = add32(b, h1);
        c = add32(c, h2);
        d = add32(d, h3);
        e = add32(e, h4);
    }

    format!(
        "{}{}{}{}{}",
        to_hex_u32(a),
        to_hex_u32(b),
        to_hex_u32(c),
        to_hex_u32(d),
        to_hex_u32(e)
    )
}

fn to_hex_u32(value: u32) -> String {
    format!("{:08x}", value)
}

fn fk(index: usize, b: u32, c: u32, d: u32) -> (u32, u32) {
    if index < 20 {
        ((b & c) | (!b & d), 0x5a827999)
    } else if index < 40 {
        (b ^ c ^ d, 0x6ed9eba1)
    } else if index < 60 {
        ((b & c) | (b & d) | (c & d), 0x8f1bbcdc)
    } else {
        (b ^ c ^ d, 0xca62c1d6)
    }
}

/// Compute the fingerprint of the given string
///
/// The output is a 64 bit number encoded as a decimal string
pub fn fingerprint(s: &str) -> u64 {
    let utf8 = s.as_bytes();
    let mut hi = hash32(utf8, utf8.len(), 0);
    let mut lo = hash32(utf8, utf8.len(), 102072);

    if hi == 0 && (lo == 0 || lo == 1) {
        hi ^= 0x130f9bef;
        lo ^= 0x949a74d4_u32.wrapping_neg();
    }

    ((hi as u64) << 32) | (lo as u64)
}

pub fn compute_msg_id(msg: &str, meaning: &str) -> String {
    let mut msg_fingerprint = fingerprint(msg);

    if !meaning.is_empty() {
        // Rotate the 64-bit message fingerprint one bit to the left and then add the meaning
        // fingerprint.
        msg_fingerprint = (msg_fingerprint << 1) | ((msg_fingerprint >> 63) & 1);
        msg_fingerprint = msg_fingerprint.wrapping_add(fingerprint(meaning));
    }

    (msg_fingerprint & 0x7FFFFFFFFFFFFFFF).to_string()
}

fn hash32(bytes: &[u8], length: usize, mut c: u32) -> u32 {
    let mut a = 0x9e3779b9u32;
    let mut b = 0x9e3779b9u32;
    let mut index = 0;

    let end = length.saturating_sub(12);
    while index <= end {
        a = a.wrapping_add(read_u32_le(bytes, index));
        b = b.wrapping_add(read_u32_le(bytes, index + 4));
        c = c.wrapping_add(read_u32_le(bytes, index + 8));
        let (na, nb, nc) = mix(a, b, c);
        a = na;
        b = nb;
        c = nc;
        index += 12;
    }

    let remainder = length - index;
    c = c.wrapping_add(length as u32);

    if remainder >= 4 {
        a = a.wrapping_add(read_u32_le(bytes, index));
        index += 4;

        if remainder >= 8 {
            b = b.wrapping_add(read_u32_le(bytes, index));
            index += 4;

            if remainder >= 9 {
                c = c.wrapping_add((read_u8(bytes, index) as u32) << 8);
                index += 1;
            }
            if remainder >= 10 {
                c = c.wrapping_add((read_u8(bytes, index) as u32) << 16);
                index += 1;
            }
            if remainder == 11 {
                c = c.wrapping_add((read_u8(bytes, index) as u32) << 24);
            }
        } else {
            if remainder >= 5 {
                b = b.wrapping_add(read_u8(bytes, index) as u32);
                index += 1;
            }
            if remainder >= 6 {
                b = b.wrapping_add((read_u8(bytes, index) as u32) << 8);
                index += 1;
            }
            if remainder == 7 {
                b = b.wrapping_add((read_u8(bytes, index) as u32) << 16);
            }
        }
    } else {
        if remainder >= 1 {
            a = a.wrapping_add(read_u8(bytes, index) as u32);
            index += 1;
        }
        if remainder >= 2 {
            a = a.wrapping_add((read_u8(bytes, index) as u32) << 8);
            index += 1;
        }
        if remainder == 3 {
            a = a.wrapping_add((read_u8(bytes, index) as u32) << 16);
        }
    }

    mix(a, b, c).2
}

fn mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(b);
    a = a.wrapping_sub(c);
    a ^= c >> 13;
    b = b.wrapping_sub(c);
    b = b.wrapping_sub(a);
    b ^= a << 8;
    c = c.wrapping_sub(a);
    c = c.wrapping_sub(b);
    c ^= b >> 13;
    a = a.wrapping_sub(b);
    a = a.wrapping_sub(c);
    a ^= c >> 12;
    b = b.wrapping_sub(c);
    b = b.wrapping_sub(a);
    b ^= a << 16;
    c = c.wrapping_sub(a);
    c = c.wrapping_sub(b);
    c ^= b >> 5;
    a = a.wrapping_sub(b);
    a = a.wrapping_sub(c);
    a ^= c >> 3;
    b = b.wrapping_sub(c);
    b = b.wrapping_sub(a);
    b ^= a << 10;
    c = c.wrapping_sub(a);
    c = c.wrapping_sub(b);
    c ^= b >> 15;
    (a, b, c)
}

#[derive(Clone, Copy)]
enum Endian {
    Little,
    Big,
}

fn add32(a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}

fn rol32(a: u32, count: u32) -> u32 {
    (a << count) | (a >> (32 - count))
}

fn bytes_to_words32(bytes: &[u8], endian: Endian) -> Vec<u32> {
    let size = (bytes.len() + 3) >> 2;
    let mut words32 = vec![0u32; size];

    for i in 0..size {
        words32[i] = word_at(bytes, i * 4, endian);
    }

    words32
}

fn byte_at(bytes: &[u8], index: usize) -> u8 {
    if index >= bytes.len() {
        0
    } else {
        bytes[index]
    }
}

fn word_at(bytes: &[u8], index: usize, endian: Endian) -> u32 {
    let mut word = 0u32;
    match endian {
        Endian::Big => {
            for i in 0..4 {
                word += (byte_at(bytes, index + i) as u32) << (24 - 8 * i);
            }
        }
        Endian::Little => {
            for i in 0..4 {
                word += (byte_at(bytes, index + i) as u32) << (8 * i);
            }
        }
    }
    word
}

fn read_u32_le(bytes: &[u8], index: usize) -> u32 {
    if index + 4 <= bytes.len() {
        u32::from_le_bytes([bytes[index], bytes[index + 1], bytes[index + 2], bytes[index + 3]])
    } else {
        0
    }
}

fn read_u8(bytes: &[u8], index: usize) -> u8 {
    if index < bytes.len() {
        bytes[index]
    } else {
        0
    }
}

