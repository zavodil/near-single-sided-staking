use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_gas::NearGas;
use near_sdk::json_types::U128;
use near_sdk::Balance;
use near_workspaces::network::Sandbox;
use near_workspaces::{types::NearToken, Account, Contract, Worker};

use near_sdk::serde_json;

// https://github.com/near/near-sdk-rs/blob/master/examples/fungible-token/tests/workspaces.rs

const CONTRACT_WASM_FILEPATH: &str = "./out/release.wasm";
const FT_WASM_FILEPATH: &str = "./out/fungible_token.wasm";
const REWARD_PER_SEC: Balance = 100_000;
const REWARD_PER_SEC_2: Balance = 2 * REWARD_PER_SEC;

const NANOSEC_IN_SEC: u64 = 1_000_000_000;

pub const TOKEN_DECIMALS: u8 = 24;
pub const TOKEN_TOTAL_SUPPLY: Balance = 1_000_000_000 * 10u128.pow(TOKEN_DECIMALS as _);

const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

fn u128_diff(a: u128, b: u128) -> u128 {
    if a > b {
        a.checked_sub(b).unwrap_or(a - b)
    } else {
        b.checked_sub(a).unwrap_or(b - a)
    }
}

async fn init(worker: &Worker<Sandbox>) -> anyhow::Result<(Contract, Contract, Account, Account)> {
    let owner = worker.dev_create_account().await?;
    let anon = worker.dev_create_account().await?;

    let ft_wasm = std::fs::read(FT_WASM_FILEPATH)?;
    let ft_contract = worker.dev_deploy(&ft_wasm).await?;

    let _ = ft_contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": owner.id().to_string(),
            "total_supply": U128::from(TOKEN_TOTAL_SUPPLY),
            "metadata": FungibleTokenMetadata {
                    spec: FT_METADATA_SPEC.to_string(),
                    name: "Token".to_string(),
                    symbol: "TOKEN".to_string(),
                    icon: None,
                    reference: None,
                    reference_hash: None,
                    decimals: TOKEN_DECIMALS,
                }
        }))
        .gas(NearGas::from_tgas(100))
        .transact()
        .await?;

    let contract_wasm = std::fs::read(CONTRACT_WASM_FILEPATH)?;
    let contract = worker.dev_deploy(&contract_wasm).await?;

    let _ = contract
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": owner.id().to_string(),
            "token_id": ft_contract.id().to_string()
        }))
        .gas(NearGas::from_tgas(100))
        .transact()
        .await?;

    // register contract in the token
    let _ = owner
        .call(ft_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": contract.id().to_string(),
        }))
        .gas(NearGas::from_tgas(100))
        .deposit(NearToken::from_yoctonear(1250000000000000000000))
        .transact()
        .await?;

    println!("contract: {:#?}", contract.id());
    println!("owner: {:#?}", owner.id());

    //near call <token_account_id> storage_deposit '{"account_id": "<contract_account_id>"}' --accountId <sender_account_id> --deposit 0.125 NEAR

    return Ok((contract, ft_contract, owner, anon));
}

#[tokio::test]
async fn verify_get_owner_id() -> anyhow::Result<()> {
    let worker: Worker<Sandbox> = near_workspaces::sandbox().await?;
    let (contract, _, owner, anon) = init(&worker).await?;

    let owner_id: serde_json::Value = contract.call("get_owner").view().await?.json()?;

    assert_eq!(owner_id, owner.id().to_string());
    assert_ne!(owner_id, contract.id().to_string());
    assert_ne!(owner_id, anon.id().to_string());

    Ok(())
}

#[tokio::test]
async fn verify_owner_ft_balance() -> anyhow::Result<()> {
    let worker: Worker<Sandbox> = near_workspaces::sandbox().await?;
    let (_, ft_contract, owner, _) = init(&worker).await?;

    let owner_ft_balance: serde_json::Value = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": owner.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    assert_eq!(owner_ft_balance, TOKEN_TOTAL_SUPPLY.to_string());
    assert_ne!(owner_ft_balance, "1".to_string());

    Ok(())
}

#[tokio::test]
async fn verify_get_set_reward_per_sec() -> anyhow::Result<()> {
    let worker: Worker<Sandbox> = near_workspaces::sandbox().await?;
    let (contract, _, owner, anon) = init(&worker).await?;

    let reward_per_nanosec: serde_json::Value = contract.call("get_reward_per_sec").view().await?.json()?;
    assert_ne!(reward_per_nanosec, REWARD_PER_SEC.to_string());

    let set_reward_per_sec_outcome_by_anon = anon
        .call(contract.id(), "modify_reward_per_sec")
        .args_json(serde_json::json!({
            "reward_per_sec": U128::from(REWARD_PER_SEC),
            "distribute_before_change": false,
        }))
        .transact()
        .await?;

    // println!("{:#?}", set_reward_per_sec_outcome_by_anon);

    assert!(!set_reward_per_sec_outcome_by_anon.is_success(), "ANON DOESN'T TRIGGER A FAILURE");

    let promise_failures = set_reward_per_sec_outcome_by_anon.receipt_failures();
    assert_eq!(promise_failures.len(), 1);
    let failure = promise_failures[0].clone().into_result();
    if let Err(err) = failure {
        assert!(
            format!("{:?}", err).contains("ERR_NOT_AN_OWNER"),
            "ANON DOESN'T TRIGGER ERR_NOT_AN_OWNER"
        );
    }

    let set_reward_per_sec_outcome_by_owner = owner
        .call(contract.id(), "modify_reward_per_sec")
        .args_json(serde_json::json!({
            "reward_per_sec": U128::from(REWARD_PER_SEC),
            "distribute_before_change": false,
        }))
        .transact()
        .await?;

    assert!(set_reward_per_sec_outcome_by_owner.is_success(), "OWNER TRIGGERED ERR_NOT_AN_OWNER");

    Ok(())
}

#[tokio::test]
async fn verify_add_rewards() -> anyhow::Result<()> {
    let worker: Worker<Sandbox> = near_workspaces::sandbox().await?;
    let (contract, ft_contract, owner, _anon) = init(&worker).await?;

    let amount_100_tokens = U128::from(NearToken::from_near(100).as_yoctonear());
    let amount_300_tokens = U128::from(NearToken::from_near(300).as_yoctonear());

    let _ = owner
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(amount_100_tokens),
            "msg": ""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    // all tokens to be returned
    let owner_ft_balance: serde_json::Value = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": owner.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    assert_eq!(
        owner_ft_balance,
        TOKEN_TOTAL_SUPPLY.to_string(),
        "NOT ALL TOKENS RETURNS AFTER INVALID ADD_REWARDS"
    );

    // add 100 tokens rewards
    let ft_transfer_outcome_add_rewards = owner
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(amount_100_tokens),
            "msg": "\"AddRewards\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    assert!(ft_transfer_outcome_add_rewards.is_success());

    let get_undistributed_reward_after_100_tokens: U128 = contract.call("get_undistributed_reward").view().await?.json()?;

    println!(
        "get_undistributed_reward_after_100_tokens: {}",
        get_undistributed_reward_after_100_tokens.0
    );

    assert!(
        (amount_100_tokens.0)
            .checked_sub(get_undistributed_reward_after_100_tokens.0)
            .unwrap()
            < REWARD_PER_SEC,
        "DIFF OF get_undistributed_reward 100 tokens IS MORE THAN A SECOND "
    );

    let owner_ft_balance_2: serde_json::Value = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": owner.id().to_string(),
        }))
        .view()
        .await?
        .json()?;
    assert_eq!(owner_ft_balance_2, (TOKEN_TOTAL_SUPPLY - amount_100_tokens.0).to_string());

    // add 300 more tokens to rewards
    let _ = owner
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(amount_300_tokens),
            "msg": "\"AddRewards\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    let get_undistributed_reward_after_300_tokens: U128 = contract.call("get_undistributed_reward").view().await?.json()?;

    println!(
        "get_undistributed_reward_after 300 + 100 tokens: {}",
        get_undistributed_reward_after_300_tokens.0
    );

    assert!(
        (amount_300_tokens.0 + amount_100_tokens.0)
            .checked_sub(get_undistributed_reward_after_300_tokens.0)
            .unwrap()
            < REWARD_PER_SEC,
        "DIFF OF get_undistributed_reward 100 tokens IS MORE THAN A SECOND "
    );

    let owner_ft_balance_3: serde_json::Value = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": owner.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    assert_eq!(
        owner_ft_balance_3,
        (TOKEN_TOTAL_SUPPLY - amount_100_tokens.0 - amount_300_tokens.0).to_string()
    );

    Ok(())
}

#[tokio::test]
async fn verify_add_deposits() -> anyhow::Result<()> {
    let worker: Worker<Sandbox> = near_workspaces::sandbox().await?;
    let (contract, ft_contract, owner, anon) = init(&worker).await?;

    let rewards_250_tokens = U128::from(NearToken::from_near(250).as_yoctonear());

    // add 250 tokens rewards
    let _ = owner
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(rewards_250_tokens),
            "msg": "\"AddRewards\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    // anon to register in the token
    let _ = anon
        .call(ft_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": anon.id().to_string(),
        }))
        .gas(NearGas::from_tgas(100))
        .deposit(NearToken::from_yoctonear(1250000000000000000000))
        .transact()
        .await?;

    // anon to receive 800 tokens
    let amount_800_tokens = U128::from(NearToken::from_near(800).as_yoctonear());
    let _ = owner
        .call(ft_contract.id(), "ft_transfer")
        .args_json(serde_json::json!({
            "receiver_id": anon.id().to_string(),
            "amount": U128::from(amount_800_tokens),
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;

    let alice = worker.dev_create_account().await?;
    println!("alice: {:#?}", alice.id());

    // alice to register in the token
    let _ = alice
        .call(ft_contract.id(), "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": alice.id().to_string(),
        }))
        .gas(NearGas::from_tgas(100))
        .deposit(NearToken::from_yoctonear(1250000000000000000000))
        .transact()
        .await?;

    // alice to receive 3000 tokens
    let amount_3000_tokens = U128::from(NearToken::from_near(3000).as_yoctonear());
    let alice_ft_deposit = owner
        .call(ft_contract.id(), "ft_transfer")
        .args_json(serde_json::json!({
            "receiver_id": alice.id().to_string(),
            "amount": U128::from(amount_3000_tokens),
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    println!("alice_ft_deposit: {:#?}", alice_ft_deposit.logs());

    // anon to stake 700 tokens
    let amount_700_tokens = U128::from(NearToken::from_near(700).as_yoctonear());
    let anon_stake = anon
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(amount_700_tokens),
            "msg": "\"Stake\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    println!("anon_stake: {:#?}", anon_stake.logs());

    let anon_get_shares: U128 = contract
        .call("get_shares")
        .args_json(serde_json::json!({
            "account_id": anon.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    assert_eq!(anon_get_shares.0, amount_700_tokens.0, "ERR ILLEGAL STAKING BALANCE");

    let anon_ft_balance: U128 = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": anon.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    assert_eq!(
        anon_ft_balance.0,
        (amount_800_tokens.0 - amount_700_tokens.0),
        "ERR ILLEGAL FT BALANCE AFTER STAKE"
    );

    // alice to stake 1400 tokens
    let amount_1400_tokens = U128::from(NearToken::from_near(1400).as_yoctonear());
    let alice_stake = alice
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(amount_1400_tokens),
            "msg": "\"Stake\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    println!("alice_stake: {:#?}", alice_stake.logs());

    let total_staked: U128 = contract.call("get_total_staked").view().await?.json()?;

    println!("total_staked: {:#?}", total_staked);

    assert_eq!(
        total_staked.0,
        amount_700_tokens.0 + amount_1400_tokens.0,
        "ERR ILLEGAL TOTAL STAKE"
    );

    // enable staking
    let set_reward_per_sec_outcome_by_owner = owner
        .call(contract.id(), "modify_reward_per_sec")
        .args_json(serde_json::json!({
            "reward_per_sec": U128::from(REWARD_PER_SEC),
            "distribute_before_change": false,
        }))
        .transact()
        .await?;
    println!(
        "set_reward_per_sec_outcome_by_owner: {:#?}",
        set_reward_per_sec_outcome_by_owner.logs()
    );

    let (timestamp_after_stake, _epoch_height): (u64, u64) = contract.call("current_env_data").view().await?.json()?;

    // start staking rewards in 1 sec from now
    let set_reset_reward_genesis_time_in_sec = owner
        .call(contract.id(), "reset_reward_genesis_time_in_sec")
        .args_json(serde_json::json!({
            "reward_genesis_time_in_sec": timestamp_after_stake / NANOSEC_IN_SEC + 1,
        }))
        .transact()
        .await?;
    println!(
        "set_reset_reward_genesis_time_in_sec: {:#?}",
        set_reset_reward_genesis_time_in_sec.logs()
    );

    //let contract_metadata = contract.call("contract_metadata").view().await?.json()?;
    //println!("contract_metadata: {:#?}", contract_metadata);

    worker.fast_forward(1000).await?;

    let (timestamp_in_1000_blocks, _epoch_height): (u64, u64) = contract.call("current_env_data").view().await?.json()?;

    let nanosec_passed = timestamp_in_1000_blocks - timestamp_after_stake;
    let sec_passed = nanosec_passed.checked_div(NANOSEC_IN_SEC).unwrap();
    println!(
        "timestamp_in_1000_blocks: {:#?}, sec passed: {}",
        timestamp_in_1000_blocks,
        sec_passed.to_string()
    );

    let anon_unstake = anon
        .call(contract.id(), "unstake")
        .args_json(serde_json::json!({}))
        .deposit(ONE_YOCTO)
        .gas(NearGas::from_tgas(100))
        .transact()
        .await?;

    println!("anon_unstake: {:#?}", anon_unstake.logs());

    let anon_get_shares_2: U128 = contract
        .call("get_shares")
        .args_json(serde_json::json!({
            "account_id": anon.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    assert_eq!(anon_get_shares_2.0, 0, "ERR ILLEGAL STAKING BALANCE");

    let anon_ft_balance_after_claim: U128 = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": anon.id().to_string(),
        }))
        .view()
        .await?
        .json()?;

    println!("anon_ft_balance_after_claim: {:#?}", anon_ft_balance_after_claim.0);

    /*let anon_rewards = (sec_passed as u128)
    .checked_mul(REWARD_PER_SEC)
    .expect("anon rewards calc overflow");*/

    let anon_rewards = (sec_passed + 1) as u128 * REWARD_PER_SEC * (amount_700_tokens.0) / (amount_1400_tokens.0 + amount_700_tokens.0);
    println!("anon_rewards: {:#?}", anon_rewards);

    let alice_rewards_round_1 =
        (sec_passed + 1) as u128 * REWARD_PER_SEC * (amount_1400_tokens.0) / (amount_1400_tokens.0 + amount_700_tokens.0);
    println!("alice_rewards: {:#?}", alice_rewards_round_1);

    // anon had 800, staked and withdrawn 700
    let anon_balance_with_rewards = amount_800_tokens.0 + anon_rewards;
    println!("anon_balance_with_rewards: {}", anon_balance_with_rewards);

    let rewards_round1_diff = u128_diff(anon_balance_with_rewards, anon_ft_balance_after_claim.0);

    println!(
        "rewards_round1_diff: {} (sec: {})",
        rewards_round1_diff,
        rewards_round1_diff / REWARD_PER_SEC
    );

    assert!(
        rewards_round1_diff <= 10 * REWARD_PER_SEC,
        "ERR ILLEGAL UNCLAIMED REWARDS (rounded to 10 sec)"
    );

    // new round of staking

    // set double rewards
    let _ = owner
        .call(contract.id(), "modify_reward_per_sec")
        .args_json(serde_json::json!({
            "reward_per_sec": U128::from(REWARD_PER_SEC_2),
            "distribute_before_change": false,
        }))
        .transact()
        .await?;

    //  anon to stake 500 more
    let amount_500_tokens = U128::from(NearToken::from_near(500).as_yoctonear());
    let anon_stake = anon
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(amount_500_tokens),
            "msg": "\"Stake\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    println!("anon_stake: {:#?}", anon_stake.logs());

    let (timestamp_after_stake_2, _epoch_height): (u64, u64) = contract.call("current_env_data").view().await?.json()?;

    // 2000 more block
    worker.fast_forward(2000).await?;

    let (timestamp_in_3000_blocks, _epoch_height): (u64, u64) = contract.call("current_env_data").view().await?.json()?;

    let nanosec_passed_2 = timestamp_in_3000_blocks - timestamp_after_stake_2;
    let sec_passed_2 = nanosec_passed_2.checked_div(NANOSEC_IN_SEC).unwrap();
    println!("sec_passed_2: {}", sec_passed_2);

    let anon_unstake_2 = anon
        .call(contract.id(), "unstake")
        .args_json(serde_json::json!({}))
        .deposit(ONE_YOCTO)
        .gas(NearGas::from_tgas(100))
        .transact()
        .await?;

    println!("anon_unstake_2: {:#?}", anon_unstake_2.logs());

    let anon_ft_balance_after_claim_round_2: U128 = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": anon.id().to_string(),
        }))
        .view()
        .await?
        .json()?;
    println!("anon_ft_balance_after_claim_round_2: {}", anon_ft_balance_after_claim_round_2.0);

    let anon_rewards_round_2 = (sec_passed_2 + 1) as u128 * REWARD_PER_SEC_2 * (amount_500_tokens.0)
        / (amount_1400_tokens.0 + alice_rewards_round_1 + amount_500_tokens.0);
    println!("anon_rewards_round_2: {:#?} anon_rewards_1: {}", anon_rewards_round_2, anon_rewards);

    // anon had 800, staked and withdrawn 500
    let anon_balance_with_rewards_2 = amount_800_tokens.0 + anon_rewards + anon_rewards_round_2;
    println!("anon_balance_with_rewards_2: {:#?}", anon_balance_with_rewards_2);

    let alice_get_shares_2: U128 = contract
        .call("get_shares")
        .args_json(serde_json::json!({
            "account_id": alice.id().to_string(),
        }))
        .view()
        .await?
        .json()?;
    println!("alice_get_shares_2: {:#?}", alice_get_shares_2);
    assert_eq!(alice_get_shares_2.0, amount_1400_tokens.0, "ERR ILLEGAL STAKING BALANCE FOR ALICE");

    let rewards_round2_diff = u128_diff(anon_balance_with_rewards_2, anon_ft_balance_after_claim_round_2.0);

    println!(
        "rewards_round2diff: {} (sec: {})",
        rewards_round2_diff,
        rewards_round2_diff / REWARD_PER_SEC_2
    );

    assert!(
        rewards_round2_diff < 10 * REWARD_PER_SEC_2,
        "ERR ILLEGAL UNCLAIMED REWARDS (rounded to 10 secs)"
    );

    // owner stake 1 token to allow alice withdraw all
    let owner_stake = owner
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": contract.id().to_string(),
            "amount": U128::from(NearToken::from_near(1).as_yoctonear()),
            "msg": "\"Stake\""
        }))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    println!("owner_stake: {:#?}", owner_stake);

    let alice_unstake = alice
        .call(contract.id(), "unstake")
        .args_json(serde_json::json!({}))
        .deposit(ONE_YOCTO)
        .gas(NearGas::from_tgas(100))
        .transact()
        .await?;
    println!("alice_unstake: {:#?}", alice_unstake.logs());

    let alice_ft_balance_after_claim_round_2: U128 = ft_contract
        .call("ft_balance_of")
        .args_json(serde_json::json!({
            "account_id": alice.id().to_string(),
        }))
        .view()
        .await?
        .json()?;
    println!("alice_ft_balance_after_claim_round_2: {}", alice_ft_balance_after_claim_round_2.0);

    // rewards in round 2 only
    let alice_rewards_round_2 = (sec_passed_2 + 1) as u128 * REWARD_PER_SEC_2 * (amount_1400_tokens.0)
        / (amount_1400_tokens.0 + alice_rewards_round_1 + amount_500_tokens.0);
    println!(
        "alice_rewards_round_2: {:#?} alice_rewards_round_1: {}",
        alice_rewards_round_2, alice_rewards_round_1
    );

    // alice had 3000, staked and withdrawn 1400
    let alice_balance_with_rewards_2 = amount_3000_tokens.0 + alice_rewards_round_1 + alice_rewards_round_2;
    println!("alice_balance_with_rewards_2: {:#?}", alice_balance_with_rewards_2);

    let alice_get_shares_3: U128 = contract
        .call("get_shares")
        .args_json(serde_json::json!({
            "account_id": alice.id().to_string(),
        }))
        .view()
        .await?
        .json()?;
    println!("alice_get_shares_3: {}", alice_get_shares_3.0);
    assert_eq!(alice_get_shares_3.0, 0, "ERR ILLEGAL STAKING BALANCE FOR ALICE after unstake");

    let alice_rewards_round2_diff = u128_diff(alice_balance_with_rewards_2, alice_ft_balance_after_claim_round_2.0);

    println!(
        "alice_rewards_round2_diff: {} (sec: {}, contains 2 diff rounds)",
        alice_rewards_round2_diff,
        alice_rewards_round2_diff / REWARD_PER_SEC_2
    );

    assert!(
        alice_rewards_round2_diff < 30 * REWARD_PER_SEC_2,
        "ERR ILLEGAL UNCLAIMED REWARDS (rounded to 30 secs)"
    );

    Ok(())
}
