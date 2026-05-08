//! Shell parser.

use super::lexer::{Token, WordPart};

#[derive(Debug, Clone)]
pub enum Ast {
    Seq(Vec<Ast>),
    AndOr(Box<Ast>, Op, Box<Ast>),
    Pipeline(Vec<Command>),
    If {
        condition: Box<Ast>,
        then_body: Box<Ast>,
        elif_clauses: Vec<(Box<Ast>, Box<Ast>)>,
        else_body: Option<Box<Ast>>,
    },
    For {
        var: String,
        items: Vec<Vec<WordPart>>,
        body: Box<Ast>,
    },
    While {
        condition: Box<Ast>,
        body: Box<Ast>,
    },
    /// Empty
    Empty,
}

#[derive(Debug, Clone, Copy)]
pub enum Op {
    And,
    Or,
}

#[derive(Debug, Clone, Default)]
pub struct Command {
    pub argv: Vec<Vec<WordPart>>,
    pub redirects: Vec<Redirect>,
    pub merge_stderr: bool,
    pub background: bool,
    pub heredoc: Option<Vec<u8>>,
    pub subshell: Option<Box<Ast>>,
}

#[derive(Debug, Clone)]
pub enum Redirect {
    Out(Vec<WordPart>),
    Append(Vec<WordPart>),
    In(Vec<WordPart>),
}

#[derive(Debug)]
pub struct ParseError(pub String);

pub fn parse(tokens: Vec<Token>) -> Result<Ast, ParseError> {
    let mut p = Parser { tokens, pos: 0 };
    let ast = p.parse_list()?;
    Ok(ast)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn skip_separators(&mut self) {
        while matches!(self.peek(), Some(Token::Newline)) {
            self.pos += 1;
        }
    }

    fn parse_list(&mut self) -> Result<Ast, ParseError> {
        let mut items: Vec<Ast> = Vec::new();
        loop {
            self.skip_separators();
            if self.peek().is_none()
                || matches!(
                    self.peek(),
                    Some(Token::RParen) | Some(Token::Fi) | Some(Token::Done) | Some(Token::Esac)
                )
            {
                break;
            }
            let item = self.parse_statement()?;
            items.push(item);
            match self.peek() {
                Some(Token::Semi) | Some(Token::Newline) | Some(Token::Background) => {
                    self.bump();
                }
                _ => {}
            }
        }
        Ok(if items.len() == 1 {
            items.pop().unwrap()
        } else if items.is_empty() {
            Ast::Empty
        } else {
            Ast::Seq(items)
        })
    }

    fn parse_statement(&mut self) -> Result<Ast, ParseError> {
        match self.peek() {
            Some(Token::If) => self.parse_if(),
            Some(Token::For) => self.parse_for(),
            Some(Token::While) => self.parse_while(),
            _ => self.parse_and_or(),
        }
    }

    fn parse_if(&mut self) -> Result<Ast, ParseError> {
        self.bump();
        let condition = Box::new(self.parse_list()?);
        if !matches!(self.peek(), Some(Token::Then)) {
            return Err(ParseError("expected 'then'".into()));
        }
        self.bump();
        let then_body = Box::new(self.parse_list()?);
        let mut elif_clauses: Vec<(Box<Ast>, Box<Ast>)> = Vec::new();
        let mut else_body: Option<Box<Ast>> = None;

        loop {
            match self.peek() {
                Some(Token::Elif) => {
                    self.bump();
                    let cond = Box::new(self.parse_list()?);
                    if !matches!(self.peek(), Some(Token::Then)) {
                        return Err(ParseError("expected 'then' after 'elif'".into()));
                    }
                    self.bump();
                    let body = Box::new(self.parse_list()?);
                    elif_clauses.push((cond, body));
                }
                Some(Token::Else) => {
                    self.bump();
                    else_body = Some(Box::new(self.parse_list()?));
                }
                Some(Token::Fi) => {
                    self.bump();
                    break;
                }
                _ => return Err(ParseError("expected 'fi'".into())),
            }
        }

        Ok(Ast::If {
            condition,
            then_body,
            elif_clauses,
            else_body,
        })
    }

    fn parse_for(&mut self) -> Result<Ast, ParseError> {
        self.bump();
        let var = match self.bump() {
            Some(Token::Word(parts)) => {
                if parts.len() == 1 {
                    if let WordPart::Lit(s) = &parts[0] {
                        s.clone()
                    } else {
                        return Err(ParseError("expected variable name after 'for'".into()));
                    }
                } else {
                    return Err(ParseError("expected variable name after 'for'".into()));
                }
            }
            _ => return Err(ParseError("expected variable name after 'for'".into())),
        };

        let mut items: Vec<Vec<WordPart>> = Vec::new();
        if matches!(self.peek(), Some(Token::In)) {
            self.bump();
            loop {
                let tok = self.peek().cloned();
                match tok {
                    Some(Token::Semi) | Some(Token::Newline) => {
                        self.bump();
                        break;
                    }
                    Some(Token::Word(parts)) => {
                        self.bump();
                        items.push(parts);
                    }
                    _ => break,
                }
            }
        }

        if !matches!(self.peek(), Some(Token::Do)) {
            return Err(ParseError("expected 'do'".into()));
        }
        self.bump();
        let body = Box::new(self.parse_list()?);
        if !matches!(self.peek(), Some(Token::Done)) {
            return Err(ParseError("expected 'done'".into()));
        }
        self.bump();

        Ok(Ast::For { var, items, body })
    }

    fn parse_while(&mut self) -> Result<Ast, ParseError> {
        self.bump();
        let condition = Box::new(self.parse_list()?);
        if !matches!(self.peek(), Some(Token::Do)) {
            return Err(ParseError("expected 'do'".into()));
        }
        self.bump();
        let body = Box::new(self.parse_list()?);
        if !matches!(self.peek(), Some(Token::Done)) {
            return Err(ParseError("expected 'done'".into()));
        }
        self.bump();

        Ok(Ast::While { condition, body })
    }

    fn parse_and_or(&mut self) -> Result<Ast, ParseError> {
        let mut left = Ast::Pipeline(self.parse_pipeline()?);
        loop {
            match self.peek() {
                Some(Token::And) => {
                    self.bump();
                    let right = Ast::Pipeline(self.parse_pipeline()?);
                    left = Ast::AndOr(Box::new(left), Op::And, Box::new(right));
                }
                Some(Token::Or) => {
                    self.bump();
                    let right = Ast::Pipeline(self.parse_pipeline()?);
                    left = Ast::AndOr(Box::new(left), Op::Or, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_pipeline(&mut self) -> Result<Vec<Command>, ParseError> {
        let mut cmds = vec![self.parse_command()?];
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.bump();
            cmds.push(self.parse_command()?);
        }
        Ok(cmds)
    }

    fn parse_command(&mut self) -> Result<Command, ParseError> {
        let mut cmd = Command::default();
        if matches!(self.peek(), Some(Token::LParen)) {
            self.bump();
            let inner = self.parse_list()?;
            if !matches!(self.peek(), Some(Token::RParen)) {
                return Err(ParseError("expected )".into()));
            }
            self.bump();
            return Ok(Command {
                argv: vec![vec![WordPart::Lit("__subshell".to_string())]],
                redirects: vec![],
                merge_stderr: false,
                background: false,
                heredoc: None,
                subshell: Some(Box::new(inner)),
            });
        }
        loop {
            match self.peek().cloned() {
                Some(Token::Word(parts)) => {
                    self.bump();
                    cmd.argv.push(parts);
                }
                Some(Token::RedirOut) => {
                    self.bump();
                    let parts = self.expect_word("expected file after >")?;
                    cmd.redirects.push(Redirect::Out(parts));
                }
                Some(Token::RedirAppend) => {
                    self.bump();
                    let parts = self.expect_word("expected file after >>")?;
                    cmd.redirects.push(Redirect::Append(parts));
                }
                Some(Token::RedirIn) => {
                    self.bump();
                    let parts = self.expect_word("expected file after <")?;
                    cmd.redirects.push(Redirect::In(parts));
                }
                Some(Token::MergeStderr) => {
                    self.bump();
                    cmd.merge_stderr = true;
                }
                Some(Token::Heredoc(_, body)) => {
                    self.bump();
                    cmd.heredoc = Some(body);
                }
                Some(Token::Background) => {
                    self.bump();
                    cmd.background = true;
                    break;
                }
                _ => break,
            }
        }
        Ok(cmd)
    }

    fn expect_word(&mut self, msg: &str) -> Result<Vec<WordPart>, ParseError> {
        match self.bump() {
            Some(Token::Word(p)) => Ok(p),
            _ => Err(ParseError(msg.to_string())),
        }
    }
}
