use anchor_lang::prelude::*;

#[error_code]
pub enum HouseError {
    #[msg("Signer is not allowed to do this")]
    Unauthorized,
    #[msg("Sales are paused")]
    Paused,
    #[msg("A parameter is out of range")]
    InvalidParams,
    #[msg("The split must add up to 10000 bps")]
    InvalidSplit,
    #[msg("The timelock has not passed yet")]
    TooEarly,
    #[msg("No split change is pending")]
    NothingPending,
    #[msg("No admin change is pending for this key")]
    NotPendingAdmin,
    #[msg("This can only be set once")]
    AlreadySet,
    #[msg("Token accounts were missing, extra, or did not match")]
    TokenAccountsMismatch,
    #[msg("Not for sale in this token")]
    NotForSale,
    #[msg("Levels are bought one at a time, in order")]
    WrongLevel,
    #[msg("The price is above the buyer's limit")]
    PriceTooHigh,
    #[msg("The guarantee is above what the prize vault may fund")]
    FundTooLarge,
    #[msg("Cups were funded too recently")]
    FundTooSoon,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("The prize vault funds only free board cups")]
    NotABoardCup,
    #[msg("The token's transfer fee is above what the house accepts")]
    TransferFeeTooHigh,
    #[msg("There is no project token yet")]
    NoToken,
    #[msg("That isn't the project token")]
    NotTheToken,
    #[msg("An auction of this token is still running")]
    AuctionRunning,
    #[msg("The last auction of this token started less than a day ago")]
    AuctionTooSoon,
    #[msg("Nothing is for sale in this auction")]
    NoAuction,
    #[msg("The lot is above a quarter of what the buyback vault holds")]
    LotTooLarge,
    #[msg("The floor is below half the last sale's price")]
    FloorTooLow,
    #[msg("The lot is smaller than the buyer's minimum")]
    LotTooSmall,
    #[msg("The buyback vault can't cover the lot")]
    VaultShort,
}
