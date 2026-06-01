use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use crate::state::{Marketplace, Offer};

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        close = buyer,
        seeds = [b"offer", marketplace.key().as_ref(), mint.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
        has_one = buyer,
        constraint = offer.mint == mint.key() @ crate::error::MarketplaceError::TokenAccountsMismatch,
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

impl<'info> CancelOffer<'info> {
    pub fn cancel(&mut self) -> Result<()> {
        // The `close = buyer` attribute handles transferring the remaining lamports (both escrowed offer amount and rent exemption) to the buyer.
        // No additional logic is required.
        Ok(())
    }
}
