use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use crate::state::{Marketplace, Listing};

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    /// CHECK: Maker will receive SOL
    pub maker: SystemAccount<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close = maker,
        seeds = [marketplace.key().as_ref(), maker.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        constraint = listing.mint == maker_mint.key() @ crate::error::MarketplaceError::TokenAccountsMismatch,
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump = marketplace.treasury_bump
    )]
    /// CHECK: Treasury
    pub treasury: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Buy<'info> {
    pub fn pay_and_transfer(&mut self) -> Result<()> {
        let price = self.listing.price;
        let fee = (price as u128 * self.marketplace.fee as u128 / 10000) as u64;
        let maker_amount = price - fee;

        // Pay Maker
        let cpi_program = self.system_program.key();
        let cpi_accounts = Transfer {
            from: self.buyer.to_account_info(),
            to: self.maker.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, maker_amount)?;

        // Pay Treasury
        let cpi_accounts_treasury = Transfer {
            from: self.buyer.to_account_info(),
            to: self.treasury.to_account_info(),
        };
        let cpi_ctx_treasury = CpiContext::new(cpi_program, cpi_accounts_treasury);
        transfer(cpi_ctx_treasury, fee)?;

        // Transfer NFT to Buyer
        let marketplace_key = self.marketplace.key();
        let maker_key = self.maker.key();
        let mint_key = self.maker_mint.key();
        
        let signer_seeds: &[&[&[u8]]] = &[&[
            marketplace_key.as_ref(),
            maker_key.as_ref(),
            mint_key.as_ref(),
            &[self.listing.bump],
        ]];

        let token_cpi_program = self.token_program.key();
        let token_cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.maker_mint.to_account_info(),
            to: self.buyer_ata.to_account_info(),
            authority: self.listing.to_account_info(),
        };

        let token_cpi_ctx = CpiContext::new_with_signer(token_cpi_program, token_cpi_accounts, signer_seeds);
        transfer_checked(token_cpi_ctx, 1, self.maker_mint.decimals)?;

        // Close Vault
        let close_cpi_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(), // give rent to maker
            authority: self.listing.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(token_cpi_program, close_cpi_accounts, signer_seeds);
        close_account(close_cpi_ctx)?;

        Ok(())
    }
}
