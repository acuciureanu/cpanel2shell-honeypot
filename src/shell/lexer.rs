//! Shell tokenizer.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A word made of one or more parts (literal, double-quoted, single-quoted, escaped).
    /// We keep parts so the expander knows where to expand `$VAR` (only outside single quotes).
    Word(Vec<WordPart>),
    Pipe,
    And,
    Or,
    Semi,
    Background,
    RedirOut,
    RedirAppend,
    RedirIn,
    Heredoc(String, Vec<u8>),
    LParen,
    RParen,
    Newline,
    MergeStderr,
    // Control flow keywords
    If,
    Then,
    Elif,
    Else,
    Fi,
    For,
    In,
    Do,
    Done,
    While,
    Until,
    Case,
    Esac,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WordPart {
    /// Raw literal — subject to var/glob expansion.
    Lit(String),
    /// Literal that should NOT be expanded (single-quoted).
    SingleQuoted(String),
    /// Double-quoted contents — subject to var/cmdsub but no glob/word-splitting.
    DoubleQuoted(String),
    /// Already escaped (backslash-prefixed) — taken literally.
    Escaped(String),
}

#[derive(Debug)]
pub struct LexError(pub String);

pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut out: Vec<Token> = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => {
                i += 1;
            }
            '\n' => {
                out.push(Token::Newline);
                i += 1;
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '|' => {
                if chars.get(i + 1) == Some(&'|') {
                    out.push(Token::Or);
                    i += 2;
                } else {
                    out.push(Token::Pipe);
                    i += 1;
                }
            }
            '&' => {
                if chars.get(i + 1) == Some(&'&') {
                    out.push(Token::And);
                    i += 2;
                } else {
                    out.push(Token::Background);
                    i += 1;
                }
            }
            ';' => {
                out.push(Token::Semi);
                i += 1;
            }
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            '>' => {
                if chars.get(i + 1) == Some(&'>') {
                    out.push(Token::RedirAppend);
                    i += 2;
                } else {
                    out.push(Token::RedirOut);
                    i += 1;
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'<') {
                    i += 2;
                    while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                        i += 1;
                    }
                    let mut delim = String::new();
                    while i < chars.len()
                        && !chars[i].is_whitespace()
                        && chars[i] != ';'
                        && chars[i] != '|'
                    {
                        delim.push(chars[i]);
                        i += 1;
                    }
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1;
                    }
                    let mut body = String::new();
                    while i < chars.len() {
                        let mut line = String::new();
                        while i < chars.len() && chars[i] != '\n' {
                            line.push(chars[i]);
                            i += 1;
                        }
                        if line == delim {
                            if i < chars.len() {
                                i += 1;
                            }
                            break;
                        }
                        body.push_str(&line);
                        body.push('\n');
                        if i < chars.len() {
                            i += 1;
                        }
                    }
                    out.push(Token::Heredoc(delim, body.into_bytes()));
                } else {
                    out.push(Token::RedirIn);
                    i += 1;
                }
            }
            // 2>&1 form
            '2' if chars.get(i + 1) == Some(&'>')
                && chars.get(i + 2) == Some(&'&')
                && chars.get(i + 3) == Some(&'1') =>
            {
                out.push(Token::MergeStderr);
                i += 4;
            }
            // 2>file (we treat as discard)
            '2' if chars.get(i + 1) == Some(&'>') => {
                i += 2;
                // skip spaces
                while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
                    i += 1;
                }
                while i < chars.len()
                    && !matches!(chars[i], ' ' | '\t' | '\n' | ';' | '|' | '&' | '>' | '<')
                {
                    i += 1;
                }
            }
            _ => {
                // word starts
                let (parts, ni) = read_word(&chars, i)?;
                // Check if it's a keyword
                if let Some(WordPart::Lit(s)) = parts.first() {
                    if parts.len() == 1 {
                        let tok = match s.as_str() {
                            "if" => Some(Token::If),
                            "then" => Some(Token::Then),
                            "elif" => Some(Token::Elif),
                            "else" => Some(Token::Else),
                            "fi" => Some(Token::Fi),
                            "for" => Some(Token::For),
                            "in" => Some(Token::In),
                            "do" => Some(Token::Do),
                            "done" => Some(Token::Done),
                            "while" => Some(Token::While),
                            "until" => Some(Token::Until),
                            "case" => Some(Token::Case),
                            "esac" => Some(Token::Esac),
                            _ => None,
                        };
                        if let Some(t) = tok {
                            out.push(t);
                            i = ni;
                            continue;
                        }
                    }
                }
                out.push(Token::Word(parts));
                i = ni;
            }
        }
    }

    Ok(out)
}

fn read_word(chars: &[char], start: usize) -> Result<(Vec<WordPart>, usize), LexError> {
    let mut parts: Vec<WordPart> = Vec::new();
    let mut buf = String::new();
    let mut i = start;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\n' | '|' | '&' | ';' | '<' | '>' => break,
            '(' | ')' => {
                // Only break if NOT preceded by '$' (i.e. not part of $(...))
                // At this point buf might end with '$' — if so, consume the group.
                if c == '(' && buf.ends_with('$') {
                    // $( ... ) — consume as part of the literal so expander handles it.
                    buf.push('(');
                    i += 1;
                    let mut depth = 1usize;
                    while i < chars.len() && depth > 0 {
                        match chars[i] {
                            '(' => {
                                depth += 1;
                                buf.push(chars[i]);
                            }
                            ')' => {
                                depth -= 1;
                                if depth > 0 {
                                    buf.push(chars[i]);
                                }
                            }
                            _ => buf.push(chars[i]),
                        }
                        i += 1;
                    }
                    buf.push(')');
                } else {
                    break;
                }
            }
            '\'' => {
                if !buf.is_empty() {
                    parts.push(WordPart::Lit(std::mem::take(&mut buf)));
                }
                i += 1;
                let mut sq = String::new();
                while i < chars.len() && chars[i] != '\'' {
                    sq.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(LexError("unmatched single quote".into()));
                }
                i += 1;
                parts.push(WordPart::SingleQuoted(sq));
            }
            '"' => {
                if !buf.is_empty() {
                    parts.push(WordPart::Lit(std::mem::take(&mut buf)));
                }
                i += 1;
                let mut dq = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        let n = chars[i + 1];
                        // bash inside double quotes: \ only escapes $ ` " \ newline
                        if matches!(n, '$' | '`' | '"' | '\\' | '\n') {
                            dq.push(n);
                            i += 2;
                            continue;
                        }
                    }
                    dq.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(LexError("unmatched double quote".into()));
                }
                i += 1;
                parts.push(WordPart::DoubleQuoted(dq));
            }
            '\\' => {
                if i + 1 < chars.len() {
                    if !buf.is_empty() {
                        parts.push(WordPart::Lit(std::mem::take(&mut buf)));
                    }
                    parts.push(WordPart::Escaped(chars[i + 1].to_string()));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                buf.push(c);
                i += 1;
            }
        }
    }
    if !buf.is_empty() {
        parts.push(WordPart::Lit(buf));
    }
    Ok((parts, i))
}
