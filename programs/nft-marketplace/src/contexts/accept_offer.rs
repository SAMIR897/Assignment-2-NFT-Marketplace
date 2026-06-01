use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use crate::state::{Marketplace, Listing, Offer};
use crate::error::MarketplaceError;

#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(mut)]
    /// CHECK: Buyer system account to receive lamports if offer is cancelled, but here we just need their key. Wait, actually we don't need buyer as account unless we want to transfer the NFT directly to their ATA, which we do.
    pub buyer: SystemAccount<'info>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        seeds = [marketplace.key().as_ref(), maker.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        constraint = listing.mint == maker_mint.key() @ MarketplaceError::TokenAccountsMismatch,
    )]
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = buyer, // excess lamports go to buyer
        seeds = [b"offer", marketplace.key().as_ref(), maker_mint.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
        has_one = buyer,
        constraint = offer.mint == maker_mint.key() @ MarketplaceError::TokenAccountsMismatch,
    )]
    pub offer: Box<Account<'info, Offer>>,

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

impl<'info> AcceptOffer<'info> {
    pub fn accept(&mut self) -> Result<()> {


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
            destination: self.maker.to_account_info(),
            authority: self.listing.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(token_cpi_program, close_cpi_accounts, signer_seeds);
        close_account(close_cpi_ctx)?;

        // Transfer escrowed SOL from offer PDA to maker and treasury
        let offer_amount = self.offer.offer_amount;
        let fee = (offer_amount as u128 * self.marketplace.fee as u128 / 10000) as u64;
        let maker_amount = offer_amount - fee;

        self.offer.sub_lamports(offer_amount)?;
        self.maker.add_lamports(maker_amount)?;
        self.treasury.add_lamports(fee)?;

        // The remaining rent exemption in offer will be transferred to buyer by the close = buyer macro.
        // The listing account will be closed and its lamports transferred to maker by the close = maker macro.

        Ok(())
    }
}
