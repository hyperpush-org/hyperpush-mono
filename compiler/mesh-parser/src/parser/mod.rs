//! Event-based parser for Mesh.
//!
//! The parser consumes a token stream and produces events (Open/Close/Advance)
//! that are later converted into a rowan green tree. This decouples parsing
//! logic from tree construction.
//!
//! # Architecture
//!
//! The parser uses matklad's event-based approach (as in rust-analyzer):
//!
//! 1. Parse functions call `open()` to start a node, `advance()` to consume
//!    tokens, and `close()` to finish a node with its actual kind.
//! 2. Events are collected into a flat `Vec<Event>`.
//! 3. `build_tree()` converts events into a rowan `GreenNode`.
//!
//! The `open_before()` method enables wrapping a previously completed node
//! (e.g., turning `ident` into `call_expr(ident, arg_list)`) using the
//! "forward parent" technique.
//!
//! # Newline Significance
//!
//! Newlines are significant in Mesh (they act as statement terminators) UNLESS
//! the parser is inside delimiters (`()`, `[]`, `{}`). The `current()` and
//! `nth()` methods transparently skip insignificant newlines. Comments are
//! always skipped by lookahead. The `advance()` method emits Advance events
//! for all skipped trivia tokens so they appear in the CST.

pub(crate) mod expressions;
pub(crate) mod items;
pub(crate) mod patterns;

use mesh_common::span::Span;
use mesh_common::token::{Token, TokenKind};

use crate::error::ParseError;
use crate::syntax_kind::SyntaxKind;

/// A parser event. Events are collected during parsing and later converted
/// into a rowan green tree by [`Parser::build_tree`].
#[derive(Debug)]
enum Event {
    /// Start a new CST node. The `kind` is initially TOMBSTONE and gets
    /// patched by `close()` with the real node kind.
    ///
    /// `forward_parent` is used by `open_before()` to indicate that this
    /// node should be opened before the node at the specified event index.
    Open {
        kind: SyntaxKind,
        forward_parent: Option<usize>,
    },
    /// Finish the current CST node.
    Close,
    /// Consume the current token, advancing the token position.
    Advance,
    /// Emit an error message (wrapped in an ERROR_NODE in the tree).
    #[allow(dead_code)]
    Error { message: String },
}

/// An opaque marker for a started but not-yet-closed CST node.
/// Contains the index into the events list where the corresponding
/// `Event::Open` was placed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MarkOpened {
    index: usize,
}

/// An opaque marker for a completed (opened and closed) CST node.
/// Used by `open_before()` to wrap a previously completed node.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MarkClosed {
    index: usize,
}

/// Event-based parser for Mesh source code.
///
/// The parser consumes a `Vec<Token>` (from the lexer) and source text,
/// producing events that are later converted into a rowan green tree.
///
/// # Usage
///
/// ```ignore
/// let tokens = mesh_lexer::Lexer::tokenize(source);
/// let mut parser = Parser::new(tokens, source);
/// // ... call parse methods ...
/// let (green, errors) = parser.build_tree();
/// ```
pub(crate) struct Parser<'src> {
    /// All tokens from the lexer (including Eof).
    tokens: Vec<Token>,
    /// Current position in the token stream.
    pos: usize,
    /// Collected parser events.
    events: Vec<Event>,
    /// Original source text (for extracting token text via spans).
    source: &'src str,
    /// Parenthesis nesting depth for newline significance.
    paren_depth: u32,
    /// Bracket nesting depth for newline significance.
    bracket_depth: u32,
    /// Brace nesting depth for newline significance.
    brace_depth: u32,
    /// Collected parse errors.
    errors: Vec<ParseError>,
    /// Whether an error has been encountered (first-error-only strategy).
    has_error: bool,
    /// When true, the Pratt postfix loop will NOT treat `DO_KW` after a call
    /// as a trailing closure. Set by control-flow parsers (`if`, `while`,
    /// `case`, `for`) around their condition/scrutinee/iterable expression
    /// so that `if fn_call() do ... end` parses `do` as the block opener.
    suppress_trailing_closure: bool,
}

impl<'src> Parser<'src> {
    /// Create a new parser from a token stream and source text.
    pub(crate) fn new(tokens: Vec<Token>, source: &'src str) -> Self {
        Self {
            tokens,
            pos: 0,
            events: Vec::new(),
            source,
            paren_depth: 0,
            bracket_depth: 0,
            brace_depth: 0,
            errors: Vec::new(),
            has_error: false,
            suppress_trailing_closure: false,
        }
    }

    // ── Lookahead ──────────────────────────────────────────────────────

    /// Returns the SyntaxKind of the current significant token.
    ///
    /// Skips over trivia tokens (comments) and insignificant newlines
    /// (newlines inside delimiters). Returns `SyntaxKind::EOF` if past
    /// the end of the token stream.
    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    /// Returns the SyntaxKind of the Nth significant token ahead.
    ///
    /// `nth(0)` is equivalent to `current()`. Skips trivia and
    /// insignificant newlines. Returns `SyntaxKind::EOF` if past end.
    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        let mut pos = self.pos;
        let mut remaining = n;
        while pos < self.tokens.len() {
            let token_kind = &self.tokens[pos].kind;
            if self.should_skip(token_kind) {
                pos += 1;
                continue;
            }
            if remaining == 0 {
                return SyntaxKind::from(token_kind.clone());
            }
            remaining -= 1;
            pos += 1;
        }
        SyntaxKind::EOF
    }

    /// Returns the text of the current significant token.
    pub(crate) fn current_text(&self) -> &str {
        self.nth_text(0)
    }

    /// Returns the text of the Nth significant token ahead.
    pub(crate) fn nth_text(&self, n: usize) -> &str {
        let mut pos = self.pos;
        let mut remaining = n;
        while pos < self.tokens.len() {
            let token_kind = &self.tokens[pos].kind;
            if self.should_skip(token_kind) {
                pos += 1;
                continue;
            }
            if remaining == 0 {
                let span = &self.tokens[pos].span;
                return &self.source[span.start as usize..span.end as usize];
            }
            remaining -= 1;
            pos += 1;
        }
        ""
    }

    /// Returns the span of the current significant token.
    pub(crate) fn current_span(&self) -> Span {
        let pos = self.skip_to_significant(self.pos);
        if pos < self.tokens.len() {
            self.tokens[pos].span
        } else {
            // Past end -- return zero-length span at end of source.
            let end = self.source.len() as u32;
            Span::new(end, end)
        }
    }

    /// Check if the current significant token matches the given kind.
    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Check if the current significant token matches any of the given kinds.
    #[allow(dead_code)]
    pub(crate) fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        let current = self.current();
        kinds.contains(&current)
    }

    // ── Mutation: node management ──────────────────────────────────────

    /// Start a new CST node. Returns a marker that must be passed to
    /// `close()` to finish the node.
    ///
    /// The node kind is initially `TOMBSTONE` and gets patched by `close()`.
    pub(crate) fn open(&mut self) -> MarkOpened {
        let mark = MarkOpened {
            index: self.events.len(),
        };
        self.events.push(Event::Open {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        mark
    }

    /// Start a new CST node BEFORE a previously completed node.
    ///
    /// This enables wrapping: e.g., after parsing `ident`, we discover it's
    /// actually a function call `ident(args)`. We use `open_before(mark_closed)`
    /// to insert an Open event before the ident node, making it a child of
    /// the new call_expr node.
    ///
    /// Uses the "forward parent" technique: instead of physically inserting
    /// into the events vec (which would invalidate indices), we set a
    /// `forward_parent` link on the completed node's Open event.
    pub(crate) fn open_before(&mut self, completed: MarkClosed) -> MarkOpened {
        let mark = MarkOpened {
            index: self.events.len(),
        };
        self.events.push(Event::Open {
            kind: SyntaxKind::TOMBSTONE,
            forward_parent: None,
        });
        // Link the completed node's Open event to point forward to this new event.
        if let Event::Open { forward_parent, .. } = &mut self.events[completed.index] {
            *forward_parent = Some(mark.index);
        }
        mark
    }

    /// Close a CST node, patching its Open event with the actual kind.
    /// Returns a `MarkClosed` that can be used with `open_before()`.
    pub(crate) fn close(&mut self, m: MarkOpened, kind: SyntaxKind) -> MarkClosed {
        if let Event::Open {
            kind: slot_kind, ..
        } = &mut self.events[m.index]
        {
            *slot_kind = kind;
        }
        self.events.push(Event::Close);
        MarkClosed { index: m.index }
    }

    // ── Mutation: token consumption ────────────────────────────────────

    /// Consume the current token, emitting Advance events for all skipped
    /// trivia tokens and then for the significant token itself.
    ///
    /// Updates delimiter depth when consuming `(`, `)`, `[`, `]`, `{`, `}`.
    pub(crate) fn advance(&mut self) {
        // Emit Advance events for any trivia tokens we skip over.
        while self.pos < self.tokens.len() && self.should_skip(&self.tokens[self.pos].kind) {
            self.events.push(Event::Advance);
            self.pos += 1;
        }

        // Emit Advance for the significant token.
        if self.pos < self.tokens.len() {
            self.update_delimiter_depth(&self.tokens[self.pos].kind.clone());
            self.events.push(Event::Advance);
            self.pos += 1;
        }
    }

    /// Consume the current token wrapped in an ERROR_NODE, advancing past it.
    /// Used when encountering an unexpected token.
    #[allow(dead_code)]
    pub(crate) fn advance_with_error(&mut self, message: &str) {
        let m = self.open();
        self.error(message);
        self.advance();
        self.close(m, SyntaxKind::ERROR_NODE);
    }

    /// If the current token matches `kind`, consume it and return true.
    /// Otherwise, emit an error and return false. Sets the error flag.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            self.error(&format!("expected {:?}", kind));
            false
        }
    }

    /// If the current token matches `kind`, consume it and return true.
    /// Otherwise, return false (no error emitted).
    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume any significant newlines (used as statement separators).
    /// Only consumes newlines that are significant (at zero delimiter depth).
    pub(crate) fn eat_newlines(&mut self) {
        while self.pos < self.tokens.len() {
            let kind = &self.tokens[self.pos].kind;
            // Skip comments (trivia)
            if matches!(
                kind,
                TokenKind::Comment | TokenKind::DocComment | TokenKind::ModuleDocComment
            ) {
                self.events.push(Event::Advance);
                self.pos += 1;
                continue;
            }
            // Only eat newlines when they are significant
            if *kind == TokenKind::Newline && !self.is_newline_insignificant() {
                self.events.push(Event::Advance);
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    // ── Error reporting ────────────────────────────────────────────────

    /// Record a parse error at the current position. Sets the error flag.
    pub(crate) fn error(&mut self, message: &str) {
        let span = self.current_span();
        self.errors.push(ParseError::new(message, span));
        self.has_error = true;
    }

    /// Record a parse error with a related span for additional context.
    pub(crate) fn error_with_related(
        &mut self,
        message: &str,
        related_span: Span,
        related_msg: &str,
    ) {
        let span = self.current_span();
        self.errors.push(ParseError::with_related(
            message,
            span,
            related_msg,
            related_span,
        ));
        self.has_error = true;
    }

    /// Whether the parser has encountered an error.
    /// Parse functions should check this and bail early.
    pub(crate) fn has_error(&self) -> bool {
        self.has_error
    }

    /// Whether trailing closures (`foo() do ... end`) are currently
    /// suppressed. Control-flow parsers set this while parsing conditions
    /// so the Pratt postfix loop does not consume `do` as a closure.
    pub(crate) fn suppress_trailing_closure(&self) -> bool {
        self.suppress_trailing_closure
    }

    // ── Newline significance ───────────────────────────────────────────

    /// Whether newlines are currently insignificant (inside delimiters).
    pub(crate) fn is_newline_insignificant(&self) -> bool {
        self.paren_depth > 0 || self.bracket_depth > 0 || self.brace_depth > 0
    }

    /// Whether a token should be skipped by lookahead methods.
    ///
    /// Comments are always skipped. Newlines are skipped when inside
    /// delimiters (insignificant context).
    fn should_skip(&self, kind: &TokenKind) -> bool {
        match kind {
            TokenKind::Comment | TokenKind::DocComment | TokenKind::ModuleDocComment => true,
            TokenKind::Newline => self.is_newline_insignificant(),
            _ => false,
        }
    }

    /// Find the position of the next significant token starting from `pos`.
    fn skip_to_significant(&self, mut pos: usize) -> usize {
        while pos < self.tokens.len() && self.should_skip(&self.tokens[pos].kind) {
            pos += 1;
        }
        pos
    }

    /// Peek past newlines and comments at the top level (outside delimiters)
    /// to find the next significant non-newline token kind.
    ///
    /// This is used for multi-line pipe continuation: after parsing an expression
    /// and seeing a NEWLINE at the top level, we peek ahead to check if `|>`
    /// follows on the next line. Returns the SyntaxKind of the first
    /// non-newline, non-comment token after the current position.
    pub(crate) fn peek_past_newlines(&self) -> SyntaxKind {
        let mut pos = self.pos;
        while pos < self.tokens.len() {
            let kind = &self.tokens[pos].kind;
            match kind {
                TokenKind::Newline
                | TokenKind::Comment
                | TokenKind::DocComment
                | TokenKind::ModuleDocComment => {
                    pos += 1;
                }
                _ => return SyntaxKind::from(kind.clone()),
            }
        }
        SyntaxKind::EOF
    }

    /// Skip newline and comment tokens at the current position, emitting
    /// Advance events for each so they appear in the CST.
    ///
    /// Used for multi-line pipe continuation: after confirming that `|>` follows
    /// newlines at the top level, consume the newlines so the Pratt parser can
    /// see the `|>` operator.
    pub(crate) fn skip_newlines_for_continuation(&mut self) {
        while self.pos < self.tokens.len() {
            match &self.tokens[self.pos].kind {
                TokenKind::Newline
                | TokenKind::Comment
                | TokenKind::DocComment
                | TokenKind::ModuleDocComment => {
                    self.events.push(Event::Advance);
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    /// Update delimiter depth when consuming a delimiter token.
    fn update_delimiter_depth(&mut self, kind: &TokenKind) {
        match kind {
            TokenKind::LParen => self.paren_depth += 1,
            TokenKind::RParen => self.paren_depth = self.paren_depth.saturating_sub(1),
            TokenKind::LBracket => self.bracket_depth += 1,
            TokenKind::RBracket => self.bracket_depth = self.bracket_depth.saturating_sub(1),
            TokenKind::LBrace => self.brace_depth += 1,
            TokenKind::RBrace => self.brace_depth = self.brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    // ── Tree building ──────────────────────────────────────────────────

    /// Convert collected events into a rowan `GreenNode` and errors.
    ///
    /// This consumes the parser. The events are processed in order,
    /// with the "forward parent" technique handling `open_before()` links.
    ///
    /// Forward parents: when `open_before(completed)` is called, the completed
    /// node's Open event gets a `forward_parent` link pointing to the wrapping
    /// Open event. During tree building, when we encounter such an Open, we
    /// follow the chain, collect all kinds, and open nodes in reverse order
    /// (outermost wrapper first). The wrapper Open events are then marked as
    /// TOMBSTONE so they are skipped when encountered later.
    pub(crate) fn build_tree(mut self) -> (rowan::GreenNode, Vec<ParseError>) {
        let mut builder = rowan::GreenNodeBuilder::new();
        let mut token_pos: usize = 0;
        let mut forward_parents: Vec<(usize, SyntaxKind)> = Vec::new();

        let mut i = 0;
        while i < self.events.len() {
            match self.events[i] {
                Event::Open {
                    kind,
                    forward_parent,
                } => {
                    if forward_parent.is_some() {
                        // Follow the forward_parent chain, collecting (index, kind) pairs.
                        forward_parents.clear();
                        let mut current = i;
                        loop {
                            let (fk, fp) = match self.events[current] {
                                Event::Open {
                                    kind,
                                    forward_parent,
                                } => (kind, forward_parent),
                                _ => unreachable!(),
                            };
                            forward_parents.push((current, fk));
                            if let Some(next) = fp {
                                current = next;
                            } else {
                                break;
                            }
                        }

                        // Mark all forward parent Open events (except the first
                        // in the chain, which is at position i) as TOMBSTONE so
                        // they are skipped when we reach them later.
                        for &(fp_idx, _) in forward_parents.iter().skip(1) {
                            if let Event::Open {
                                ref mut kind,
                                ref mut forward_parent,
                            } = self.events[fp_idx]
                            {
                                *kind = SyntaxKind::TOMBSTONE;
                                *forward_parent = None;
                            }
                        }
                        // Clear the forward_parent on the first event too.
                        if let Event::Open {
                            ref mut forward_parent,
                            ..
                        } = self.events[i]
                        {
                            *forward_parent = None;
                        }

                        // Open nodes in reverse order: outermost wrapper first.
                        for &(_, fk) in forward_parents.iter().rev() {
                            if fk != SyntaxKind::TOMBSTONE {
                                builder.start_node(rowan::SyntaxKind(fk as u16));
                            }
                        }
                    } else if kind != SyntaxKind::TOMBSTONE {
                        builder.start_node(rowan::SyntaxKind(kind as u16));
                    }
                    // TOMBSTONE nodes are silently skipped.
                }
                Event::Close => {
                    builder.finish_node();
                }
                Event::Advance => {
                    if token_pos < self.tokens.len() {
                        let token = &self.tokens[token_pos];
                        let syntax_kind = SyntaxKind::from(token.kind.clone());
                        let text = &self.source[token.span.start as usize..token.span.end as usize];
                        builder.token(rowan::SyntaxKind(syntax_kind as u16), text);
                        token_pos += 1;
                    }
                }
                Event::Error { .. } => {
                    // Errors are tracked in self.errors; the node wrapping
                    // is handled by advance_with_error using Open/Close events.
                }
            }
            i += 1;
        }

        (builder.finish(), self.errors)
    }
}

// ── Top-level parsing ──────────────────────────────────────────────────

/// Parse a complete source file.
///
/// Opens a SOURCE_FILE node, parses items and statements until EOF, and
/// closes the root node.
pub(crate) fn parse_source_file(p: &mut Parser) {
    let root = p.open();

    loop {
        p.eat_newlines();
        while p.eat(SyntaxKind::SEMICOLON) {
            p.eat_newlines();
        }

        if p.at(SyntaxKind::EOF) {
            break;
        }

        parse_item_or_stmt(p);

        if p.has_error() {
            // On error, skip to next newline or EOF.
            while !p.at(SyntaxKind::NEWLINE) && !p.at(SyntaxKind::EOF) {
                p.advance();
            }
            break;
        }

        // After a statement, handle separators.
        match p.current() {
            SyntaxKind::NEWLINE => {
                p.eat_newlines();
            }
            SyntaxKind::SEMICOLON => {
                // Will be eaten at top of loop.
            }
            SyntaxKind::EOF => break,
            _ => {}
        }
    }

    // Consume remaining tokens (including EOF).
    while !p.at(SyntaxKind::EOF) {
        p.advance();
    }
    p.advance(); // EOF

    p.close(root, SyntaxKind::SOURCE_FILE);
}

/// Parse an item or statement.
///
/// Dispatches based on the current token to either an item parser (fn, module,
/// import, struct) or a statement/expression parser.
pub(crate) fn parse_item_or_stmt(p: &mut Parser) {
    match p.current() {
        // pub -> peek ahead for item keyword
        SyntaxKind::PUB_KW => match p.nth(1) {
            SyntaxKind::FN_KW | SyntaxKind::DEF_KW => items::parse_fn_def(p),
            SyntaxKind::MODULE_KW => items::parse_module_def(p),
            SyntaxKind::STRUCT_KW => items::parse_struct_def(p),
            SyntaxKind::INTERFACE_KW => items::parse_interface_def(p),
            SyntaxKind::SUPERVISOR_KW => items::parse_supervisor_def(p),
            SyntaxKind::TYPE_KW if p.nth(2) == SyntaxKind::IDENT => {
                // pub type Name ... -- disambiguate sum type vs type alias
                let mut lookahead = 3; // past PUB, TYPE_KW, IDENT
                if p.nth(lookahead) == SyntaxKind::LT {
                    lookahead += 1; // past <
                    let mut depth = 1u32;
                    while depth > 0 {
                        match p.nth(lookahead) {
                            SyntaxKind::LT => {
                                depth += 1;
                                lookahead += 1;
                            }
                            SyntaxKind::GT => {
                                depth -= 1;
                                lookahead += 1;
                            }
                            SyntaxKind::EOF => break,
                            _ => {
                                lookahead += 1;
                            }
                        }
                    }
                }
                if p.nth(lookahead) == SyntaxKind::DO_KW {
                    items::parse_sum_type_def(p);
                } else {
                    items::parse_type_alias(p);
                }
            }
            _ => {
                p.error("expected `fn`, `module`, `struct`, `interface`, `type`, or `supervisor` after `pub`");
            }
        },

        // fn/def: named function definition (fn + IDENT) vs closure (fn + L_PAREN/ARROW)
        SyntaxKind::FN_KW | SyntaxKind::DEF_KW => {
            // Disambiguate: if next token is IDENT, it's a named fn def.
            // Otherwise it's a closure expression.
            if p.nth(1) == SyntaxKind::IDENT {
                items::parse_fn_def(p);
            } else {
                expressions::expr(p);
            }
        }

        SyntaxKind::MODULE_KW => items::parse_module_def(p),

        SyntaxKind::IMPORT_KW => items::parse_import_decl(p),

        // "from" is an IDENT, not a keyword -- check text
        SyntaxKind::IDENT if p.current_text() == "from" => {
            items::parse_from_import_decl(p);
        }

        SyntaxKind::STRUCT_KW => items::parse_struct_def(p),

        SyntaxKind::INTERFACE_KW => items::parse_interface_def(p),

        SyntaxKind::IMPL_KW => items::parse_impl_def(p),

        SyntaxKind::ACTOR_KW => items::parse_actor_def(p),

        SyntaxKind::SERVICE_KW => items::parse_service_def(p),

        SyntaxKind::SUPERVISOR_KW => items::parse_supervisor_def(p),

        // type at top level followed by IDENT -> sum type def or type alias
        // Distinguish: type Name do ... end (sum type) vs type Name = ... (alias)
        // Also handles generics: type Name<T> do/= ...
        SyntaxKind::TYPE_KW if p.nth(1) == SyntaxKind::IDENT => {
            // Look past name and optional generic params to find `do` or `=`
            let mut lookahead = 2; // past TYPE_KW and IDENT
            if p.nth(lookahead) == SyntaxKind::LT {
                // Skip generic params: <T, U, ...>
                lookahead += 1; // past <
                let mut depth = 1u32;
                while depth > 0 {
                    match p.nth(lookahead) {
                        SyntaxKind::LT => {
                            depth += 1;
                            lookahead += 1;
                        }
                        SyntaxKind::GT => {
                            depth -= 1;
                            lookahead += 1;
                        }
                        SyntaxKind::EOF => break,
                        _ => {
                            lookahead += 1;
                        }
                    }
                }
            }
            if p.nth(lookahead) == SyntaxKind::DO_KW {
                items::parse_sum_type_def(p);
            } else {
                items::parse_type_alias(p);
            }
        }

        SyntaxKind::LET_KW => expressions::parse_let_binding(p),

        SyntaxKind::RETURN_KW => expressions::parse_return_expr(p),

        _ => {
            expressions::expr(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh_lexer::Lexer;

    #[test]
    fn smoke_test_parser_produces_green_node() {
        let source = "let x = 5";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        // Open a SOURCE_FILE node.
        let root = parser.open();

        // Open a LET_BINDING node.
        let binding = parser.open();

        // Consume: let, x, =, 5
        parser.advance(); // let
        parser.advance(); // x
        parser.advance(); // =
        parser.advance(); // 5

        parser.close(binding, SyntaxKind::LET_BINDING);

        // Consume EOF.
        parser.advance(); // Eof

        parser.close(root, SyntaxKind::SOURCE_FILE);

        let (green, errors) = parser.build_tree();
        assert!(errors.is_empty(), "expected no errors: {:?}", errors);

        // Verify the green node has the right kind.
        let root_node = crate::cst::SyntaxNode::new_root(green);
        assert_eq!(root_node.kind(), SyntaxKind::SOURCE_FILE);

        // Verify the text is preserved.
        assert_eq!(root_node.text().to_string(), "letx=5");
        // Note: text is "letx=5" because the lexer strips whitespace and
        // the parser emits tokens without whitespace. The CST text is the
        // concatenation of token texts. Whitespace reconstruction would
        // need additional logic.

        // Verify child structure.
        let children: Vec<_> = root_node.children().collect();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].kind(), SyntaxKind::LET_BINDING);
    }

    #[test]
    fn parser_newline_significance_inside_parens() {
        // Inside parentheses, newlines should be invisible to current()/nth().
        let source = "(\n42\n)";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        // Before consuming '(', newlines are significant.
        assert_eq!(parser.current(), SyntaxKind::L_PAREN);

        // Consume '(' -- now inside parens, newlines become insignificant.
        let root = parser.open();
        parser.advance(); // (

        // current() should skip the newline and see INT_LITERAL.
        assert_eq!(parser.current(), SyntaxKind::INT_LITERAL);

        parser.advance(); // 42

        // Should skip another newline and see ')'.
        assert_eq!(parser.current(), SyntaxKind::R_PAREN);

        parser.advance(); // )
        parser.advance(); // Eof
        parser.close(root, SyntaxKind::SOURCE_FILE);

        let (green, errors) = parser.build_tree();
        assert!(errors.is_empty());

        let root_node = crate::cst::SyntaxNode::new_root(green);
        assert_eq!(root_node.kind(), SyntaxKind::SOURCE_FILE);
    }

    #[test]
    fn parser_newline_significant_at_top_level() {
        // At top level (no delimiters), newlines are significant.
        let source = "x\ny";
        let tokens = Lexer::tokenize(source);
        let parser = Parser::new(tokens, source);

        // First token is IDENT.
        assert_eq!(parser.current(), SyntaxKind::IDENT);

        // nth(1) should be NEWLINE (significant at top level).
        assert_eq!(parser.nth(1), SyntaxKind::NEWLINE);

        // nth(2) should be the second IDENT.
        assert_eq!(parser.nth(2), SyntaxKind::IDENT);
    }

    #[test]
    fn parser_comments_are_trivia() {
        // Comments should be invisible to current()/nth().
        let source = "x # this is a comment\ny";
        let tokens = Lexer::tokenize(source);
        let parser = Parser::new(tokens, source);

        assert_eq!(parser.current(), SyntaxKind::IDENT);
        // nth(1) should skip the comment and see NEWLINE.
        assert_eq!(parser.nth(1), SyntaxKind::NEWLINE);
        // nth(2) should see the second IDENT.
        assert_eq!(parser.nth(2), SyntaxKind::IDENT);
    }

    #[test]
    fn parser_expect_success_and_failure() {
        let source = "let x";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        let root = parser.open();

        // Expecting LET_KW should succeed.
        assert!(parser.expect(SyntaxKind::LET_KW));
        assert!(!parser.has_error());

        // Expecting LET_KW again should fail (current is IDENT).
        assert!(!parser.expect(SyntaxKind::LET_KW));
        assert!(parser.has_error());
        assert_eq!(parser.errors.len(), 1);

        parser.advance(); // x
        parser.advance(); // Eof
        parser.close(root, SyntaxKind::SOURCE_FILE);

        let (_green, errors) = parser.build_tree();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn parser_eat_consumes_on_match() {
        let source = "let x";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        // eat LET_KW should succeed.
        assert!(parser.eat(SyntaxKind::LET_KW));

        // eat LET_KW should fail (no error emitted).
        assert!(!parser.eat(SyntaxKind::LET_KW));
        assert!(!parser.has_error());
    }

    #[test]
    fn parser_open_before_wraps_completed_node() {
        // Demonstrate wrapping: parse an ident, then wrap it in a call_expr.
        let source = "f(42)";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        let root = parser.open();

        // Parse the function name.
        let name = parser.open();
        parser.advance(); // f (Ident)
        let name_closed = parser.close(name, SyntaxKind::NAME_REF);

        // Now wrap it: open_before the name to create CALL_EXPR.
        let call = parser.open_before(name_closed);
        parser.advance(); // (
        parser.advance(); // 42
        parser.advance(); // )
        parser.close(call, SyntaxKind::CALL_EXPR);

        parser.advance(); // Eof
        parser.close(root, SyntaxKind::SOURCE_FILE);

        let (green, errors) = parser.build_tree();
        assert!(errors.is_empty());

        let root_node = crate::cst::SyntaxNode::new_root(green);
        assert_eq!(root_node.kind(), SyntaxKind::SOURCE_FILE);

        // The CALL_EXPR should contain NAME_REF as a child.
        let call_node = root_node.children().next().unwrap();
        assert_eq!(call_node.kind(), SyntaxKind::CALL_EXPR);

        let name_ref = call_node.children().next().unwrap();
        assert_eq!(name_ref.kind(), SyntaxKind::NAME_REF);
    }

    #[test]
    fn parser_current_text_returns_token_text() {
        let source = "hello 42";
        let tokens = Lexer::tokenize(source);
        let parser = Parser::new(tokens, source);

        assert_eq!(parser.current_text(), "hello");
    }

    #[test]
    fn parser_eat_newlines_consumes_significant_newlines() {
        let source = "x\n\ny";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        parser.advance(); // x
        parser.eat_newlines(); // consume both newlines

        // After eating newlines, current should be IDENT (y).
        assert_eq!(parser.current(), SyntaxKind::IDENT);
    }

    #[test]
    fn parser_delimiter_depth_tracking() {
        let source = "([{x}])";
        let tokens = Lexer::tokenize(source);
        let mut parser = Parser::new(tokens, source);

        let root = parser.open();

        assert_eq!(parser.paren_depth, 0);
        parser.advance(); // (
        assert_eq!(parser.paren_depth, 1);

        parser.advance(); // [
        assert_eq!(parser.bracket_depth, 1);

        parser.advance(); // {
        assert_eq!(parser.brace_depth, 1);

        parser.advance(); // x

        parser.advance(); // }
        assert_eq!(parser.brace_depth, 0);

        parser.advance(); // ]
        assert_eq!(parser.bracket_depth, 0);

        parser.advance(); // )
        assert_eq!(parser.paren_depth, 0);

        parser.advance(); // Eof
        parser.close(root, SyntaxKind::SOURCE_FILE);

        let (_green, _errors) = parser.build_tree();
    }
}
