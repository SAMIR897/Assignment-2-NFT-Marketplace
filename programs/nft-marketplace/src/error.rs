use anchor_lang::prelude::*;

#[error_code]
pub enum MarketplaceError {
    #[msg("Name must be between 1 and 32 characters")]
    NameTooLong,
    #[msg("Invalid fee. Fee must be between 0 and 10000")]
    InvalidFee,
    #[msg("Price must be greater than zero")]
    InvalidPrice,
    #[msg("Payment mint does not match the listing")]
    InvalidPaymentMint,
    #[msg("Insufficient funds to complete the purchase")]
    InsufficientFunds,
    #[msg("Offer amount must be greater than zero")]
    InvalidOffer,
    #[msg("Token accounts mismatch")]
    TokenAccountsMismatch,
}
