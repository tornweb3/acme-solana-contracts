//! Moving value into and out of escrow, for SOL and SPL tokens alike, including
//! Token-2022 mints with a transfer fee (the project token: a StonkFun mint).
//!
//! A room or cup PDA is its own escrow: it holds the lamports of a SOL stake,
//! and is the authority of the associated token account that holds a token
//! stake. Every payment out goes through `Escrow::pay`, which checks that the
//! destination really belongs to the intended recipient before moving anything,
//! so no caller can forget the check that stops a stranger being paid.
//!
//! Transfer fees. A deposit is grossed up so the escrow receives exactly the
//! stake (the payer pays the tax); a payment out moves exactly the amount owed
//! from escrow (the recipient nets it less the tax). Withheld fees are harvested
//! to the mint before an escrow account is closed. New deposits are refused when
//! the mint's current OR scheduled fee is above the cap set for it, because the
//! fee authority (not us) can raise it; payments out are never refused.
//!
//! Required memos. A Token-2022 account's owner can turn on required memos
//! (`MemoTransfer`) at any time, after joining too; a payment into such an
//! account is preceded by a memo through the memo program (`MEMO_PROGRAM_ID`,
//! passed as the last remaining account), so nobody can stop a refund or a
//! settlement that pays everyone by turning it on.

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke},
    system_program,
};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_2022::spl_token_2022::{
        extension::{
            memo_transfer::memo_required, transfer_fee::TransferFeeConfig, BaseStateWithExtensions,
            ExtensionType, StateWithExtensions,
        },
        state::{Account as SplAccount, Mint as SplMint},
    },
    token_2022_extensions::transfer_fee::{
        harvest_withheld_tokens_to_mint, transfer_checked_with_fee, HarvestWithheldTokensToMint,
        TransferCheckedWithFee,
    },
    token_interface::{self, CloseAccount, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::{constants::*, error::ArenaError};

/// Only accept mints whose transfers the escrow can account for exactly.
/// A permanent delegate could drain the pot without the PDA signing; a transfer
/// hook, a pausable or default-frozen mint could block refunds. A transfer fee is
/// accepted only when `allow_transfer_fee` (the escrow then grosses deposits up
/// and harvests before closing). Unknown extensions fail closed.
pub fn validate_wager_mint(mint: &AccountInfo, allow_transfer_fee: bool) -> Result<bool> {
    if *mint.owner == anchor_spl::token::ID {
        return Ok(false);
    }
    require_keys_eq!(
        *mint.owner,
        anchor_spl::token_2022::ID,
        ArenaError::UnsupportedMint
    );
    let data = mint.try_borrow_data()?;
    let state = StateWithExtensions::<SplMint>::unpack(&data)?;
    let mut has_fee = false;
    for extension in state.get_extension_types()? {
        match extension {
            ExtensionType::MintCloseAuthority
            | ExtensionType::MetadataPointer
            | ExtensionType::TokenMetadata
            | ExtensionType::GroupPointer
            | ExtensionType::TokenGroup
            | ExtensionType::GroupMemberPointer
            | ExtensionType::TokenGroupMember => {}
            ExtensionType::TransferFeeConfig if allow_transfer_fee => has_fee = true,
            _ => return err!(ArenaError::UnsupportedMint),
        }
    }
    Ok(has_fee)
}

/// The mint's transfer fee config, if it has one.
fn fee_config(mint: &AccountInfo) -> Result<Option<TransferFeeConfig>> {
    if *mint.owner != anchor_spl::token_2022::ID {
        return Ok(None);
    }
    let data = mint.try_borrow_data()?;
    let state = StateWithExtensions::<SplMint>::unpack(&data)?;
    Ok(state.get_extension::<TransferFeeConfig>().ok().copied())
}

/// A transfer-fee mint's fee in force this epoch and its scheduled one, in bps
/// (None: no transfer fee). The house checks its own cap with it.
pub fn transfer_fee_bps(mint: &AccountInfo) -> Result<Option<(u16, u16)>> {
    let Some(cfg) = fee_config(mint)? else {
        return Ok(None);
    };
    let now = u16::from(
        cfg.get_epoch_fee(Clock::get()?.epoch)
            .transfer_fee_basis_points,
    );
    let newer = u16::from(cfg.newer_transfer_fee.transfer_fee_basis_points);
    Ok(Some((now, newer)))
}

/// Both the fee in force this epoch and the scheduled one must be at most
/// `max_bps` (the older one stops counting once the newer has taken over).
pub fn check_fee_cap(mint: &AccountInfo, max_bps: u16) -> Result<()> {
    if let Some((now, newer)) = transfer_fee_bps(mint)? {
        require!(
            now <= max_bps && newer <= max_bps,
            ArenaError::TransferFeeTooHigh
        );
    }
    Ok(())
}

/// A Token-2022 account whose owner requires memos on incoming transfers.
fn requires_memo(account: &AccountInfo) -> Result<bool> {
    if *account.owner != anchor_spl::token_2022::ID {
        return Ok(false);
    }
    let data = account.try_borrow_data()?;
    let state = StateWithExtensions::<SplAccount>::unpack(&data)?;
    Ok(memo_required(&state))
}

/// The memo program off the end of an instruction's remaining accounts, when
/// it is there: (the accounts before it, the memo program).
pub fn split_memo<'a, 'info>(
    accounts: &'a [AccountInfo<'info>],
) -> (&'a [AccountInfo<'info>], Option<AccountInfo<'info>>) {
    match accounts.split_last() {
        Some((last, rest)) if *last.key == MEMO_PROGRAM_ID => (rest, Some(last.clone())),
        _ => (accounts, None),
    }
}

/// Raw token-account fields shared by Token and Token-2022: mint, owner, amount.
pub fn token_account_fields(
    info: &AccountInfo,
    token_program: &Pubkey,
) -> Result<(Pubkey, Pubkey, u64)> {
    require_keys_eq!(
        *info.owner,
        *token_program,
        ArenaError::PayoutAccountsMismatch
    );
    let data = info.try_borrow_data()?;
    require!(data.len() >= 72, ArenaError::PayoutAccountsMismatch);
    let mint =
        Pubkey::try_from(&data[0..32]).map_err(|_| error!(ArenaError::PayoutAccountsMismatch))?;
    let owner =
        Pubkey::try_from(&data[32..64]).map_err(|_| error!(ArenaError::PayoutAccountsMismatch))?;
    let amount = u64::from_le_bytes(data[64..72].try_into().unwrap());
    Ok((mint, owner, amount))
}

/// The token side of a stake: present for SPL, absent for SOL.
pub struct TokenSide<'info> {
    pub mint: AccountInfo<'info>,
    pub mint_key: Pubkey,
    pub decimals: u8,
    pub escrow: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    /// The mint carries a transfer fee (its transfers go through the fee path).
    pub transfer_fee: bool,
}

/// Check the optional token accounts an instruction was given against the
/// stake: all absent for SOL; all present, of the stake's mint and program,
/// with the escrow being the PDA's associated token account, for SPL.
pub fn token_side<'info>(
    stake_mint: &Pubkey,
    stake_token_program: &Pubkey,
    transfer_fee: bool,
    pda: &Pubkey,
    mint: Option<&InterfaceAccount<'info, Mint>>,
    escrow: Option<&InterfaceAccount<'info, TokenAccount>>,
    token_program: &Option<Interface<'info, TokenInterface>>,
) -> Result<Option<TokenSide<'info>>> {
    if *stake_mint == NATIVE_SOL {
        require!(
            mint.is_none() && escrow.is_none() && token_program.is_none(),
            ArenaError::TokenAccountsMismatch
        );
        return Ok(None);
    }
    let (Some(mint), Some(escrow), Some(token_program)) = (mint, escrow, token_program) else {
        return err!(ArenaError::TokenAccountsMismatch);
    };
    require_keys_eq!(mint.key(), *stake_mint, ArenaError::TokenAccountsMismatch);
    require_keys_eq!(
        token_program.key(),
        *stake_token_program,
        ArenaError::TokenAccountsMismatch
    );
    require_keys_eq!(
        escrow.key(),
        get_associated_token_address_with_program_id(pda, stake_mint, stake_token_program),
        ArenaError::TokenAccountsMismatch
    );
    Ok(Some(TokenSide {
        mint: mint.to_account_info(),
        mint_key: mint.key(),
        decimals: mint.decimals,
        escrow: escrow.to_account_info(),
        token_program: token_program.to_account_info(),
        transfer_fee,
    }))
}

/// The gross amount to send so that `net` arrives after the mint's fee, and that fee.
fn gross_for_net(mint: &AccountInfo, net: u64) -> Result<(u64, u64)> {
    let Some(cfg) = fee_config(mint)? else {
        return Ok((net, 0));
    };
    let epoch = Clock::get()?.epoch;
    let tf = cfg.get_epoch_fee(epoch);
    let mut gross = tf
        .calculate_pre_fee_amount(net)
        .ok_or(ArenaError::Overflow)?;
    // The pre-fee amount is exact by construction; step up once if rounding ever
    // disagrees, and let the deposit's balance check be the final word.
    for _ in 0..2 {
        let fee = tf.calculate_fee(gross).ok_or(ArenaError::Overflow)?;
        if gross.checked_sub(fee) == Some(net) {
            return Ok((gross, fee));
        }
        gross = gross.checked_add(1).ok_or(ArenaError::Overflow)?;
    }
    err!(ArenaError::DepositMismatch)
}

/// The fee the mint takes on a transfer of `amount` now.
fn fee_on(mint: &AccountInfo, amount: u64) -> Result<u64> {
    let Some(cfg) = fee_config(mint)? else {
        return Ok(0);
    };
    cfg.calculate_epoch_fee(Clock::get()?.epoch, amount)
        .ok_or_else(|| error!(ArenaError::Overflow))
}

/// Take exactly `amount` from a player into escrow: SOL by transfer, a token by
/// `transfer_checked` (grossed up by the fee for a transfer-fee mint). The
/// escrow's balance must grow by exactly `amount`.
pub fn deposit<'info>(
    payer: &AccountInfo<'info>,
    pda: &AccountInfo<'info>,
    token: &Option<TokenSide<'info>>,
    payer_tokens: Option<&InterfaceAccount<'info, TokenAccount>>,
    system_program: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    match token {
        None => {
            require!(payer_tokens.is_none(), ArenaError::TokenAccountsMismatch);
            system_program::transfer(
                CpiContext::new(
                    system_program.key(),
                    system_program::Transfer {
                        from: payer.clone(),
                        to: pda.clone(),
                    },
                ),
                amount,
            )
        }
        Some(t) => {
            let from = payer_tokens.ok_or(ArenaError::TokenAccountsMismatch)?;
            let before = token_account_fields(&t.escrow, t.token_program.key)?.2;
            if t.transfer_fee {
                let (gross, fee) = gross_for_net(&t.mint, amount)?;
                transfer_checked_with_fee(
                    CpiContext::new(
                        t.token_program.key(),
                        TransferCheckedWithFee {
                            token_program_id: t.token_program.clone(),
                            source: from.to_account_info(),
                            mint: t.mint.clone(),
                            destination: t.escrow.clone(),
                            authority: payer.clone(),
                        },
                    ),
                    gross,
                    t.decimals,
                    fee,
                )?;
            } else {
                token_interface::transfer_checked(
                    CpiContext::new(
                        t.token_program.key(),
                        TransferChecked {
                            from: from.to_account_info(),
                            mint: t.mint.clone(),
                            to: t.escrow.clone(),
                            authority: payer.clone(),
                        },
                    ),
                    amount,
                    t.decimals,
                )?;
            }
            let after = token_account_fields(&t.escrow, t.token_program.key)?.2;
            require!(
                after.checked_sub(before) == Some(amount),
                ArenaError::DepositMismatch
            );
            Ok(())
        }
    }
}

/// A PDA acting as escrow, with the seeds it signs with.
pub struct Escrow<'a, 'info> {
    pub pda: AccountInfo<'info>,
    pub seeds: &'a [&'a [u8]],
    pub token: Option<TokenSide<'info>>,
    /// The memo program, for payments into accounts that require memos.
    pub memo: Option<AccountInfo<'info>>,
}

impl<'a, 'info> Escrow<'a, 'info> {
    /// Pay `amount` to `recipient` through `to`: for SOL `to` IS the recipient's
    /// wallet; for tokens it is a token account of the stake's mint OWNED by the
    /// recipient. Checked even for zero amounts. For a transfer-fee mint the
    /// escrow is debited exactly `amount` and the recipient nets it less the fee.
    pub fn pay(&self, to: &AccountInfo<'info>, recipient: &Pubkey, amount: u64) -> Result<()> {
        match &self.token {
            None => {
                require_keys_eq!(*to.key, *recipient, ArenaError::PayoutAccountsMismatch);
                transfer_from_pda(&self.pda, to, amount)
            }
            Some(t) => {
                let (mint, owner, _) = token_account_fields(to, t.token_program.key)?;
                require_keys_eq!(mint, t.mint_key, ArenaError::PayoutAccountsMismatch);
                require_keys_eq!(owner, *recipient, ArenaError::PayoutAccountsMismatch);
                if amount == 0 {
                    return Ok(());
                }
                require!(
                    self.token_balance()? >= amount,
                    ArenaError::InsufficientEscrow
                );
                // (the memo must come just before the transfer it is for)
                if requires_memo(to)? {
                    let memo = self.memo.as_ref().ok_or(ArenaError::MemoRequired)?;
                    invoke(
                        &Instruction {
                            program_id: MEMO_PROGRAM_ID,
                            accounts: vec![],
                            data: b"ACME payout".to_vec(),
                        },
                        core::slice::from_ref(memo),
                    )?;
                }
                if t.transfer_fee {
                    let fee = fee_on(&t.mint, amount)?;
                    transfer_checked_with_fee(
                        CpiContext::new_with_signer(
                            t.token_program.key(),
                            TransferCheckedWithFee {
                                token_program_id: t.token_program.clone(),
                                source: t.escrow.clone(),
                                mint: t.mint.clone(),
                                destination: to.clone(),
                                authority: self.pda.clone(),
                            },
                            &[self.seeds],
                        ),
                        amount,
                        t.decimals,
                        fee,
                    )
                } else {
                    token_interface::transfer_checked(
                        CpiContext::new_with_signer(
                            t.token_program.key(),
                            TransferChecked {
                                from: t.escrow.clone(),
                                mint: t.mint.clone(),
                                to: to.clone(),
                                authority: self.pda.clone(),
                            },
                            &[self.seeds],
                        ),
                        amount,
                        t.decimals,
                    )
                }
            }
        }
    }

    pub fn token_balance(&self) -> Result<u64> {
        match &self.token {
            None => Ok(0),
            Some(t) => Ok(token_account_fields(&t.escrow, t.token_program.key)?.2),
        }
    }

    /// Token stakes only: send anything still in the escrow (a donation, a
    /// reward pushed to it) to `surplus_to` (owned by `surplus_owner`; needed only
    /// when there is something), harvest
    /// the transfer fees withheld in it to the mint, then close it, its rent to
    /// `rent_to`.
    pub fn close_token_escrow(
        &self,
        surplus_to: Option<&AccountInfo<'info>>,
        surplus_owner: &Pubkey,
        rent_to: &AccountInfo<'info>,
    ) -> Result<()> {
        let Some(t) = &self.token else {
            return Ok(());
        };
        let residue = self.token_balance()?;
        if residue > 0 {
            let to = surplus_to.ok_or(ArenaError::TokenAccountsMismatch)?;
            self.pay(to, surplus_owner, residue)?;
        }
        if t.transfer_fee {
            harvest_withheld_tokens_to_mint(
                CpiContext::new(
                    t.token_program.key(),
                    HarvestWithheldTokensToMint {
                        token_program_id: t.token_program.clone(),
                        mint: t.mint.clone(),
                    },
                ),
                vec![t.escrow.clone()],
            )?;
        }
        token_interface::close_account(CpiContext::new_with_signer(
            t.token_program.key(),
            CloseAccount {
                account: t.escrow.clone(),
                destination: rent_to.clone(),
                authority: self.pda.clone(),
            },
            &[self.seeds],
        ))
    }
}

/// Move lamports out of a program-owned PDA by direct debit/credit, with checked
/// arithmetic, and never below the PDA's rent-exempt minimum.
pub fn transfer_from_pda<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let rent_floor = Rent::get()?.minimum_balance(from.data_len());
    let remaining = from
        .lamports()
        .checked_sub(amount)
        .ok_or(ArenaError::InsufficientEscrow)?;
    require!(remaining >= rent_floor, ArenaError::InsufficientEscrow);
    let credited = to
        .lamports()
        .checked_add(amount)
        .ok_or(ArenaError::Overflow)?;
    **from.try_borrow_mut_lamports()? = remaining;
    **to.try_borrow_mut_lamports()? = credited;
    Ok(())
}

/// The lamports a program-owned PDA holds above its rent-exempt minimum: what
/// is left in a SOL escrow once everything owed has been paid (windfalls).
pub fn lamports_above_rent(pda: &AccountInfo) -> Result<u64> {
    let floor = Rent::get()?.minimum_balance(pda.data_len());
    Ok(pda.lamports().saturating_sub(floor))
}
