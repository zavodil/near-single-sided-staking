NEAR Single Sided Staking Contract
======
Based on [XREF Token Contract](https://github.com/ref-finance/ref-token/tree/master/xref-token) , but this contract has been modified to avoid creating a new token, performing only the staking functionality. Additionally, the latest SDK is utilized here, and [near-workspaces](https://github.com/near/near-workspaces-rs/) tests have been added.


### Sumary
* Stake a given token to lock in the contract and get staking shares

* Redeem shares by unstake.

* Anyone can add a given as reward.  
 
* Admin to set `reward_per_sec` as a total reward for all stakers proportionally.
 
* Admin can modify `reward_genesis_time_in_sec` before it passed.


### How to start

Compile 
```
./build_local.sh 
```

Deploy
```
near deploy <contract_account_id> --wasmFile ./out/release.wasm
```

Init
```
near call new pub '{"owner_id": <sender_account_id>, "token_id": <token_account_id>) --accountId <contract_account_id>
```

Register staking contract in the token contract
```
near call <token_account_id> storage_deposit '{"account_id": "<contract_account_id>"}' --accountId <sender_account_id> --deposit 0.125 NEAR
```

### Management


#### stake
```bash
near call <token_account_id> ft_transfer_call '{"receiver_id": "'<contract_account_id>'", "amount": "10''", "msg": "\"Stake\""}' --account_id=<user_account_id> --amount=$YN --gas=$GAS100
```

#### Add tokens as reward
```bash
near call <token_account_id> ft_transfer_call '{"receiver_id": "'<contract_account_id>'", "amount": "10''", "msg": "\"AddRewards\""}' --account_id=<user_account_id> --amount=$YN --gas=$GAS100
```

#### Unstake, get token and reward back
```bash
near call <contract_account_id> unstake '{"amount": "8''"}' --account_id=<user_account_id> --amount=$YN --gas=$GAS100
```

#### Owner reset reward genesis time
```bash
# set to 2022-01-22 01:00:00 UTC time
near call <contract_account_id> reset_reward_genesis_time_in_sec '{"reward_genesis_time_in_sec": 1642813200}' --account_id=<sender_account_id>
```
Note: would return false if already past old genesis time or the new genesis time is a past time.

#### Owner modify reward_per_sec
```bash
near call <contract_account_id> modify_reward_per_sec '{"reward_per_sec": "1''", "distribute_before_change": true}' --account_id=<sender_account_id> --gas=$GAS100
```
Note: If `distribute_before_change` is true, contract will sync up reward distribution using the old `reward_per_sec` at call time before changing to the new one.


### HOW TO RUN TESTS


All tests 
```
cargo test -- --nocapture
```

Test by function name
```
cargo test --test main verify_add_deposits -- --nocapture
```


### BUILD DOCKER ON M1:

Prepare docker
```
 clone https://github.com/near/near-sdk-rs/pull/720/files
 ./build_docker_m1.sh
```

Run docker buildx `contract-builder`
``` 
 ./build_docker_m1.sh
```

