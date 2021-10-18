use evercity_runtime::pallet_evercity::account::{
    EvercityAccountStructT, AUDITOR_ROLE_MASK, CUSTODIAN_ROLE_MASK, IMPACT_REPORTER_ROLE_MASK,
    INVESTOR_ROLE_MASK, ISSUER_ROLE_MASK, MANAGER_ROLE_MASK, MASTER_ROLE_MASK,
};
use evercity_runtime::pallet_evercity_accounts;

use evercity_runtime::{
    AccountId, AuraConfig, BalancesConfig, EvercityConfig, GenesisConfig, GrandpaConfig, Signature,
    SudoConfig, SystemConfig, EvercityAccountsConfig, WASM_BINARY,
};
use sp_core::{crypto::Ss58Codec, sr25519, Pair, Public};

type EvercityAccountStruct = EvercityAccountStructT<u64>;

use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![authority_keys_from_seed("Alice")],
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        MASTER_ROLE_MASK,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        CUSTODIAN_ROLE_MASK,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Charlie"),
                        ISSUER_ROLE_MASK,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Dave"),
                        INVESTOR_ROLE_MASK,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Eve"),
                        AUDITOR_ROLE_MASK,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                        MANAGER_ROLE_MASK,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Evercity"),
                        IMPACT_REPORTER_ROLE_MASK,
                    ),
                ],
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        pallet_evercity_accounts::accounts::MASTER_ROLE_MASK,
                    ),
                ],
                // Sudo account
                get_account_id_from_seed::<sr25519::Public>("Alice"),
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary =
        WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            let master_account_id: AccountId =
                Ss58Codec::from_ss58check("5DJBx8EcrJqWqDQDe3xPd7Bw2zL3obvHigdLZKVGDHx7GRwW")
                    .unwrap();

            testnet_genesis(
                wasm_binary,
                // @FIXME! setup Master and Custodian
                vec![authority_keys_from_seed("Evercity//Master")],
                vec![(master_account_id.clone(), MASTER_ROLE_MASK)],
                vec![(master_account_id.clone(), pallet_evercity_accounts::accounts::MASTER_ROLE_MASK)],
                // Sudo account
                master_account_id,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules.
#[allow(clippy::redundant_clone)]
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    endowed_accounts: Vec<(AccountId, u8)>,
    evercity_accounts: Vec<(AccountId, pallet_evercity_accounts::accounts::RoleMask)>,
    _root_key: AccountId,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .map(|x| (x.0.clone(), 1 << 60))
                .collect(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        pallet_sudo: Some(SudoConfig { key: _root_key }),
        pallet_evercity: Some(EvercityConfig {
            // set roles for each pre-set accounts (set role)
            genesis_account_registry: endowed_accounts
                .iter()
                .map(|(acc, role)| {
                    (
                        acc.clone(),
                        EvercityAccountStruct {
                            roles: *role,
                            identity: 0,
                            create_time: 0,
                        },
                    )
                })
                .collect(),
        }),
        pallet_evercity_accounts: Some(EvercityAccountsConfig {
            // set roles for each pre-set accounts (set role)
            genesis_account_registry: evercity_accounts
                .iter()
                .map(|(acc, role)| {
                    (
                        acc.clone(),
                        pallet_evercity_accounts::accounts::AccountStruct {
                            roles: *role,
                        },
                    )
                })
                .collect(),
        }),
    }
}
