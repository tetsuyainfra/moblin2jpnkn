use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

#[derive(Debug, Default)]
pub struct Stanza {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<String>,
}

impl Stanza {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    s.rsplit(':').next().unwrap_or("").to_string()
}

fn set_root(stanza: &mut Stanza, e: &BytesStart) -> Result<(), String> {
    stanza.name = local(e.name().as_ref());
    for a in e.attributes().flatten() {
        let k = String::from_utf8_lossy(a.key.as_ref()).to_string();
        let v = a.unescape_value().map_err(|e| e.to_string())?.to_string();
        stanza.attrs.push((k, v));
    }
    Ok(())
}

/// WebSocketの1フレーム（単一のXMPPスタンザ）を解析する。
pub fn parse(input: &str) -> Result<Stanza, String> {
    let mut reader = Reader::from_str(input);
    let mut stanza = Stanza::default();
    let mut depth = 0usize;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                if stanza.name.is_empty() {
                    set_root(&mut stanza, &e)?;
                } else if depth == 1 {
                    stanza.children.push(local(e.name().as_ref()));
                }
                depth += 1;
            }
            Event::Empty(e) => {
                if stanza.name.is_empty() {
                    set_root(&mut stanza, &e)?;
                } else if depth == 1 {
                    stanza.children.push(local(e.name().as_ref()));
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Text(t) if depth == 1 => {
                stanza.text.push_str(&t.unescape().map_err(|e| e.to_string())?);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    if stanza.name.is_empty() {
        return Err("empty XML".into());
    }
    Ok(stanza)
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_open() {
        let s = parse(r#"<open xmlns="urn:ietf:params:xml:ns:xmpp-framing" to="x" version="1.0"/>"#).unwrap();
        assert_eq!(s.name, "open");
        assert_eq!(s.attr("to"), Some("x"));
    }

    #[test]
    fn parse_auth() {
        let s = parse(r#"<auth xmlns="urn:ietf:params:xml:ns:xmpp-sasl" mechanism="PLAIN">AG1vYmxpbgA=</auth>"#).unwrap();
        assert_eq!(s.attr("mechanism"), Some("PLAIN"));
        assert_eq!(s.text, "AG1vYmxpbgA=");
    }

    #[test]
    fn parse_iq_children() {
        let s = parse(r#"<iq type="set" id="a"><bind xmlns="b"><resource>r</resource></bind><session xmlns="s"/></iq>"#).unwrap();
        assert_eq!(s.children, vec!["bind", "session"]);
    }
}
