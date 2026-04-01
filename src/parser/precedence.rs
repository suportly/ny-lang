use crate::lexer::token::TokenKind;

// Precedence table (lowest to highest):
//   ||            (1, 2)   logical OR
//   &&            (3, 4)   logical AND
//   == != < > <= >=  (5, 6) comparison
//   |             (7, 8)   bitwise OR
//   ^             (9, 10)  bitwise XOR
//   & (infix)     (11, 12) bitwise AND
//   << >>         (13, 14) shifts
//   + -           (15, 16) additive
//   * / %         (17, 18) multiplicative
//   unary - ! ~ & *  (_, 19) prefix
//   as . [] ()        postfix (handled in parse_expr loop)
pub fn infix_bp(kind: &TokenKind) -> Option<(u8, u8)> {
    match kind {
        TokenKind::Or => Some((1, 2)),
        TokenKind::And => Some((3, 4)),
        TokenKind::Eq
        | TokenKind::Ne
        | TokenKind::Lt
        | TokenKind::Gt
        | TokenKind::Le
        | TokenKind::Ge => Some((5, 6)),
        TokenKind::Pipe => Some((7, 8)),
        TokenKind::Caret => Some((9, 10)),
        TokenKind::Ampersand => Some((11, 12)),
        TokenKind::LtLt | TokenKind::GtGt => Some((13, 14)),
        TokenKind::Plus | TokenKind::Minus => Some((15, 16)),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((17, 18)),
        _ => None,
    }
}

/// Returns right binding power for prefix operators.
pub fn prefix_bp(kind: &TokenKind) -> Option<u8> {
    match kind {
        TokenKind::Minus | TokenKind::Not | TokenKind::Tilde => Some(19),
        _ => None,
    }
}
