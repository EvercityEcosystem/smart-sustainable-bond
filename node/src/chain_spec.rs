use sp_core::{Pair, Public, sr25519, crypto::{Ss58Codec, Ss58AddressFormat}};
use evercity_runtime::{
    AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig,
    SudoConfig, SystemConfig, WASM_BINARY, Signature, EvercityConfig
};

use evercity_runtime::evercity::{MASTER_ROLE_MASK, CUSTODIAN_ROLE_MASK, EMITENT_ROLE_MASK, INVESTOR_ROLE_MASK};

use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{Verify, IdentifyAccount};
use sc_service::ChainType;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s),
    )
}


pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || testnet_genesis(
            wasm_binary,
            // Initial PoA authorities
            vec![
                authority_keys_from_seed("Alice"),
            ],
            // Sudo account
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            // Pre-funded accounts
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            ],
            true,
        ),
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
    let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || testnet_genesis(
            wasm_binary,
            // Initial PoA authorities
            vec![
                authority_keys_from_seed("Alice"),
                authority_keys_from_seed("Bob"),
            ],
            // Sudo account
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            // Pre-funded accounts
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Charlie"),
                get_account_id_from_seed::<sr25519::Public>("Dave"),
                get_account_id_from_seed::<sr25519::Public>("Eve"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
            ],
            true,
        ),
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
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    
    let _pre_master_account_id: AccountId = 
        Ss58Codec::from_ss58check("5DJBx8EcrJqWqDQDe3xPd7Bw2zL3obvHigdLZKVGDHx7GRwW").unwrap();
    let _pre_custodian_account_id: AccountId = 
        Ss58Codec::from_ss58check("5EZ4JyMCxR6k5oDPAV5Bh1hqvMreqLZMbaXX2XUTk6f3ZPDL").unwrap();
    let _pre_emitent_account_id: AccountId = 
        Ss58Codec::from_ss58check("5FxdLBFRrE7NF3u2Tq95XE5gM1ve4YAd9ZnP8ZujUJ85gf7c").unwrap();
    let _pre_investor_account_id: AccountId = 
        Ss58Codec::from_ss58check("5FzuNtedbrnQrsKZKpAUzRy6swX9hM1PiLemREKoN2tBc3W1").unwrap();


    GenesisConfig {
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts.iter().cloned().map(|k|(k, 1 << 60)).collect(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
        }),
        pallet_sudo: Some(SudoConfig {
            key: root_key,
        }),
        evercity: Some(EvercityConfig {
            // set roles for each pre-set accounts (set role)
            genesis_accounts: [
                (_pre_master_account_id.clone(), MASTER_ROLE_MASK),
                (_pre_custodian_account_id.clone(), CUSTODIAN_ROLE_MASK),
                (_pre_emitent_account_id.clone(), EMITENT_ROLE_MASK),
                (_pre_investor_account_id.clone(), INVESTOR_ROLE_MASK),
            ].iter().cloned().collect(),
            
            // set value (temporary - bool) for each specific map of accounts
            genesis_master_accounts: [(_pre_master_account_id.clone(), true)].iter().cloned().collect(),
            genesis_custodian_accounts: [(_pre_custodian_account_id.clone(), true)].iter().cloned().collect(),
            genesis_emitent_accounts: [(_pre_emitent_account_id.clone(), true)].iter().cloned().collect(),
            genesis_investor_accounts: [(_pre_investor_account_id.clone(), true)].iter().cloned().collect(),
        }),
    }
}
