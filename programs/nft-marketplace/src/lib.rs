use anchor_lang::prelude::*;

pub mod contexts;
pub mod state;
pub mod error;

pub use contexts::*;

declare_id!("DCyS1on55VpDooLA7oyZHbMn5J9iiVgoAJAX9TLWi7XL"); // Will be updated by anchor keys sync

#[program]
pub mod nft_marketplace {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name: String, fee: u16) -> Result<()> {
        ctx.accounts.init(name, fee, &ctx.bumps)
    }

    pub fn list(ctx: Context<List>, price: u64, payment_mint: Option<Pubkey>) -> Result<()> {
        ctx.accounts.create_listing(price, payment_mint, &ctx.bumps)?;
        ctx.accounts.deposit_nft()
    }

    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        ctx.accounts.withdraw_nft()
    }

    pub fn buy(ctx: Context<Buy>) -> Result<()> {
        ctx.accounts.pay_and_transfer()
    }

    pub fn buy_with_token(ctx: Context<BuyWithToken>) -> Result<()> {
        ctx.accounts.pay_and_transfer()
    }

    pub fn make_offer(ctx: Context<MakeOffer>, offer_amount: u64) -> Result<()> {
        ctx.accounts.process(offer_amount, &ctx.bumps)
    }

    pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        ctx.accounts.accept()
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        ctx.accounts.cancel()
    }
}
