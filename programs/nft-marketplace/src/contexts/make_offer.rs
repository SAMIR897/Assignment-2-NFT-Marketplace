use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_interface::Mint;
use crate::state::{Marketplace, Offer};
use crate::error::MarketplaceError;

#[derive(Accounts)]
#[instruction(offer_amount: u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = buyer,
        space = Offer::INIT_SPACE,
        seeds = [b"offer", marketplace.key().as_ref(), mint.key().as_ref(), buyer.key().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    pub system_program: Program<'info, System>,
}

impl<'info> MakeOffer<'info> {
    pub fn process(&mut self, offer_amount: u64, bumps: &MakeOfferBumps) -> Result<()> {
        require!(offer_amount > 0, MarketplaceError::InvalidOffer);

        self.offer.set_inner(Offer {
            buyer: self.buyer.key(),
            mint: self.mint.key(),
            payment_mint: None, // SOL only for this challenge, but could be extended
            offer_amount,
            bump: bumps.offer,
        });

        // Transfer SOL to the Offer PDA to escrow
        let cpi_program = self.system_program.key();
        let cpi_accounts = Transfer {
            from: self.buyer.to_account_info(),
            to: self.offer.to_account_info(), // The PDA itself holds the escrowed SOL
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, offer_amount)?;

        Ok(())
    }
}
