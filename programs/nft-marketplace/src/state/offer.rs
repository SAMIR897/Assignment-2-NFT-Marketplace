use anchor_lang::prelude::*;

#[account]
pub struct Offer {
    pub buyer: Pubkey,
    pub mint: Pubkey,
    pub payment_mint: Option<Pubkey>, // None = SOL, Some = SPL token
    pub offer_amount: u64,
    pub bump: u8,
}

impl Space for Offer {
    const INIT_SPACE: usize = 8 + 32 + 32 + (1 + 32) + 8 + 1;
}
