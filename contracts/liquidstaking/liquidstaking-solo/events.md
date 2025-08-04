# Events

List of events that are emitted by liquid staking contract that is captured by indexer

## bond

event attribute key: bond

This event is captured when user stake/bond to contract

| event key | description |
|--|--|
| sender | address that call the contract  |
| staker| address that bond/stake |
| channel_id|  source ucs03 channel_id of staker  |
| bond_amount | amount that is staked/bonded  |
| output_amount| liquid staking amount that is returned to staker  |
| delegated_amount| total amount that is delegated to validators include bond_amount from user |
| total_bond_amount| total delegated_amount + total rewards include bond_amount from user |
| total_supply| total ebaby/liquid staking amount includes new minted amount  |
| exchange_rate | exchange rate when user bond/stake |
| time | timestamp of bond transaction  |
| denom | native/staking denom  |
| recipient | recipient address (in hex if recipient_channel_id is not 0)  |
| recipient_channel_id | ucs03 channel id of recipient (0 if recipient is on same chain) |
| reward_balance | total withdrawed reward balance on contract  |
| unclaimed_reward | unclaimed reward that is not yet withdrawed |
| ibc_channel_id | source ibc channel id of staker that bond via IBC  |


## unbond request

event attribute key: unstake_request

This event is emitted when user submit unbond request as part of specific batch

  | event key | description |
|--|--|
| sender | address that call the contract |
| staker | user address that request to unbond/unstake |
| channel_id | source ucs03 channel_id of staker |
| unbond_amount | liquid staking token amount that will be unbonded |
| time | timestamp of unbond request transaction |
| batch_id | id of undelegation/unbond batch |
| record_id| unbond request record id |
| recipient | recipient address that will get back the token (in hex if recipient_channel_id is not 0)  |
| recipient_channel_id | ucs03 channel id of recipient (0 if recipient is on same chain) |
| reward_balance | total withdrawed reward balance on contract  |


## submit batch

event attribute key: submit_batch

This event is emitted when periodic time of batch submission is met so the current pending batch is submitted by backend scheduler, this means contract will call undelegate requests to validators.
Batch will be submitted every 8 hours.

| event key | description |
|--|--|
| batch_id | batch id that is submitted |
| sender | address that call the contract |
| unstake_amount | total liquid staking token that is unstaked in batch |
| output_amount | total native token that will be unbonded in batch |
| delegated_amount| total amount that is delegated to validators after it is substracted by total amount that is undelegated in batch  |
| total_bond_amount | total delegated_amount + total rewards |
| total_supply | total ebaby/liquid staking amount that already substracted by total unstake amount  |
| exchange_rate | exchange rate when batch is submitted |
| time | timestamp of transaction |
| denom | native denom name |

## process batch unbonding

event attribute key: process_batch_unbonding

This event is emitted when batch unbond period is over (complete unbonding) so batch process will release the native token for unbond requests as part of batch. 
Max unbond requests that will be processed is 50, so if the unbond requests in batch is more than 50, this event will be emitted a few times per batch

| event key | description |
|--|--|
| total_amount | total native token amount of all unbond requests in batch  |
| released_amount | total liquid staking amount that is released as part of batch |
| denom | denom that will be received after unbonding |
| time | timestamp of unbonding transaction  |
| batch_id | batch id that will process the unbonding |
| record_ids | array of unbond request record id that is part of batch |

## process_unbonding

event attribute key: process_unbonding

This event is emitted per user recipient address when unbonding batch is processed as batch unbonding period is over (complete unbonding), this event will be emitted once per user and contains information of amount that will be sent to user.

| event key | description |
|--|--|
| staker | original address that call the unbond request |
| amount | amount that will be sent to user/recipient |
| denom | denom that will be received after unbonding |
| time | timestamp of unbonding transaction  |
| batch_id | batch id that is processed |
| channel_id | source ucs03 channel_id of staker |
| recipient | recipient address that will get back the token (in hex if recipient_channel_id is not 0)  |
| recipient_channel_id | ucs03 channel id of recipient (0 if recipient is on same chain) |


## batch received

event attribute key: "batch_received";

This event will be emitted when complete unbonding is done and contract get total amount from validators that is received by contract 

| event key | description |
|--|--|
| batch_id | batch id of unbond batch that receive unbonded token from validators |
| received_amount | total amount that are undelegated  (complete unbonding)  and receivedfrom validators of the batch |
| time | timestamp of transaction when batch is released completely   |


## batch released

event attribute key: batch_released;

This event will be emitted when all unbond records of batch are released already

| event key | description |
|--|--|
| batch_id | batch id that is finished as all unbond records of the batch are released |
| time | timestamp of transaction when batch is released completely   |