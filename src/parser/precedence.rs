use crate::lexer::token::TokenKind;

/// Returns (left binding power, right binding power) for infix operators.
/// Higher binding power = higher precedence.
/// Left < Right = left-associative.
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
        TokenKind::Plus | TokenKind::Minus => Some((7, 8)),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((9, 10)),
        _ => None,
    }
}

/// Returns right binding power for prefix operators.
pub fn prefix_bp(kind: &TokenKind) -> Option<u8> {
    match kind {
        TokenKind::Minus | TokenKind::Not => Some(11),
        _ => None,
    }
}
