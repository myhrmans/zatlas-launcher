//! Steam's text KeyValues files (.vdf, .acf): quoted keys, quoted values and braces. Just enough to read them.

pub enum Vdf {
    Str(String),
    Map(Vec<(String, Vdf)>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Map(entries) => entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Map(entries) => entries,
            Vdf::Str(_) => &[],
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Map(_) => None,
        }
    }
}

enum Token {
    Str(String),
    Open,
    Close,
}

fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Token::Open),
            '}' => out.push(Token::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => {}
                        },
                        c => s.push(c),
                    }
                }
                out.push(Token::Str(s));
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The whole file as one map; anything malformed ends the map it's in.
pub fn parse(text: &str) -> Vdf {
    fn map(tokens: &mut std::iter::Peekable<std::vec::IntoIter<Token>>) -> Vdf {
        let mut entries = Vec::new();
        while let Some(Token::Str(key)) = tokens.next() {
            match tokens.next() {
                Some(Token::Str(value)) => entries.push((key, Vdf::Str(value))),
                Some(Token::Open) => entries.push((key, map(tokens))),
                _ => break,
            }
        }
        Vdf::Map(entries)
    }
    map(&mut tokens(text).into_iter().peekable())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_maps() {
        let v = parse(
            "\"AppWorkshop\"\n{\n\t\"appid\"\t\t\"221100\"\n\t\"WorkshopItemsInstalled\"\n\t{\n\t\t\"1559212036\"\n\t\t{\n\t\t\t\"size\"\t\t\"538560\"\n\t\t}\n\t}\n}\n",
        );
        let w = v.get("AppWorkshop").unwrap();
        assert_eq!(w.get("appid").unwrap().as_str(), Some("221100"));
        let item = &w.get("WorkshopItemsInstalled").unwrap().entries()[0];
        assert_eq!(item.0, "1559212036");
        assert_eq!(item.1.get("size").unwrap().as_str(), Some("538560"));
    }

    #[test]
    fn reads_escaped_paths() {
        let v = parse(r#""libraryfolders" { "0" { "path" "C:\\Program Files (x86)\\Steam" } }"#);
        let path = v
            .get("libraryfolders")
            .unwrap()
            .get("0")
            .unwrap()
            .get("path")
            .unwrap()
            .as_str();
        assert_eq!(path, Some(r"C:\Program Files (x86)\Steam"));
    }
}
