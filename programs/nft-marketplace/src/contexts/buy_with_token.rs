use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use crate::state::{Marketplace, Listing};
use crate::error::MarketplaceError;

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    /// CHECK: Maker will receive SPL token
    pub maker: SystemAccount<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    pub payment_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
    )]
    pub maker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = maker_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_nft_ata: Box<InterfaceAccount<'info, TokenAccount>>,

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
        seeds = [b"treasury", marketplace.key().as_ref()],
        bump = marketplace.treasury_bump
    )]
    /// CHECK: Treasury
    pub treasury: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = treasury,
    )]
    pub treasury_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> BuyWithToken<'info> {
    pub fn pay_and_transfer(&mut self) -> Result<()> {
        require!(self.listing.payment_mint == Some(self.payment_mint.key()), MarketplaceError::InvalidPaymentMint);

        let price = self.listing.price;
        let fee = (price as u128 * self.marketplace.fee as u128 / 10000) as u64;
        let maker_amount = price - fee;

        // Pay Maker with SPL Token
        let cpi_program = self.token_program.key();
        let cpi_accounts = TransferChecked {
            from: self.buyer_payment_ata.to_account_info(),
            mint: self.payment_mint.to_account_info(),
            to: self.maker_payment_ata.to_account_info(),
            authority: self.buyer.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, maker_amount, self.payment_mint.decimals)?;

        // Pay Treasury with SPL Token
        let cpi_accounts_treasury = TransferChecked {
            from: self.buyer_payment_ata.to_account_info(),
            mint: self.payment_mint.to_account_info(),
            to: self.treasury_payment_ata.to_account_info(),
            authority: self.buyer.to_account_info(),
        };
        let cpi_ctx_treasury = CpiContext::new(cpi_program, cpi_accounts_treasury);
        transfer_checked(cpi_ctx_treasury, fee, self.payment_mint.decimals)?;

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

        let token_cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.maker_mint.to_account_info(),
            to: self.buyer_nft_ata.to_account_info(),
            authority: self.listing.to_account_info(),
        };

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, token_cpi_accounts, signer_seeds);
        transfer_checked(token_cpi_ctx, 1, self.maker_mint.decimals)?;

        // Close Vault
        let close_cpi_accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.listing.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(cpi_program, close_cpi_accounts, signer_seeds);
        close_account(close_cpi_ctx)?;

        Ok(())
    }
}
