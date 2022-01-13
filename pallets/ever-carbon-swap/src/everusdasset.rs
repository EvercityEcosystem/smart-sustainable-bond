use frame_support::{
    codec::{Decode, Encode},
    sp_runtime::RuntimeDebug,
};

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, PartialEq)]
pub struct EverUSDAssetMinRequest<AccountId, AssetId, AssetBalance> {
    pub account: AccountId,
    pub asset_id: AssetId,
    pub count_to_mint: AssetBalance
}

impl<AccountId, AssetId, AssetBalance> EverUSDAssetMinRequest<AccountId, AssetId, AssetBalance> {
    pub fn new(account: AccountId, asset_id: AssetId, count_to_mint: AssetBalance) -> Self {
        Self {
            account,
            asset_id,
            count_to_mint,
        }
    }
}