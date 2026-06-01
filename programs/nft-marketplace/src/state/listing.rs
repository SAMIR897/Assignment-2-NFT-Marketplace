use anchor_lang::prelude::*;

#[account]
pub struct Listing {
    pub maker: Pubkey,
    pub mint: Pubkey,
    pub payment_mint: Option<Pubkey>, // None = SOL, Some = SPL Token
    pub price: u64,
    pub bump: u8,
}

impl Space for Listing {
    const INIT_SPACE: usize = 8 + 32 + 32 + (1 + 32) + 8 + 1;
}
