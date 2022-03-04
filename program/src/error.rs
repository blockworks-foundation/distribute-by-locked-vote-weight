use anchor_lang::prelude::*;

#[error]
pub enum ErrorKind {
    // 6000 / 0x1770
    #[msg("")]
    SomeError,
}
