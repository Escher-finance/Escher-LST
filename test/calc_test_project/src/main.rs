use cosmwasm_std::{Decimal, Uint128};
use std::fmt;

/// In this example:
/// - We assume a "base APR" of 10% per year.
/// - Of that 10%, the validator charges a 5% commission on *the rewards*, not on the entire stake.
///   So effectively, the user sees ~9.5% net if the entire year passes.
/// - We'll store a "time factor" to simulate partial-year or multi-year progression.

/// State tracks:
/// - total_bond_amount: total underlying tokens staked
/// - total_supply: total minted staking tokens
/// - exchange_rate: ratio = total_bond_amount / total_supply
/// - annual_apr: e.g., Decimal("0.10") for 10%
/// - validator_commission: e.g., Decimal("0.05") for 5%
#[derive(Clone)]
pub struct State {
    pub total_bond_amount: Uint128,
    pub total_supply: Uint128,
    pub exchange_rate: Decimal,
    pub annual_apr: Decimal,
    pub validator_commission: Decimal,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "State {{ total_bond_amount: {}, total_supply: {}, exchange_rate: {}, annual_apr: {}, validator_commission: {} }}",
            self.total_bond_amount, self.total_supply, self.exchange_rate,
            self.annual_apr, self.validator_commission
        )
    }
}

impl State {
    /// Create a new state with no tokens, an exchange rate of 1.0,
    /// a default 10% APR, and a 5% validator commission
    pub fn new() -> Self {
        Self {
            total_bond_amount: Uint128::zero(),
            total_supply: Uint128::zero(),
            exchange_rate: Decimal::one(),
            annual_apr: Decimal::from_ratio(1u128, 10u128),  // 0.10
            validator_commission: Decimal::from_ratio(1u128, 20u128), // 0.05
        }
    }

    /// Recompute the exchange_rate = total_bond_amount / total_supply,
    /// except if total_supply == 0 or total_bond_amount == 0, then use 1.0
    pub fn update_exchange_rate(&mut self) {
        if self.total_bond_amount.is_zero() || self.total_supply.is_zero() {
            self.exchange_rate = Decimal::one();
        } else {
            self.exchange_rate =
                Decimal::from_ratio(self.total_bond_amount, self.total_supply);
        }
    }

    /// Simulate "deposit" of some underlying tokens.
    /// minted = floor( deposit / exchange_rate )
    pub fn deposit(&mut self, native_amount: Uint128) -> Uint128 {
        println!(
            "\n[deposit] user depositing {} underlying tokens...",
            native_amount
        );
        println!("[deposit] old state: {:?}", self);

        let deposit_dec = Decimal::from_ratio(native_amount, Uint128::one());
        // minted = deposit / exchange_rate
        let minted_dec = deposit_dec / self.exchange_rate;
        let minted = minted_dec.to_uint_floor();

        // Update state
        self.total_bond_amount += native_amount;
        self.total_supply += minted;

        self.update_exchange_rate();

        println!("[deposit] minted: {}", minted);
        println!("[deposit] new state: {:?}", self);

        minted
    }

    /// Simulate "withdraw" of some staking tokens.
    /// underlying = floor( staking_tokens * exchange_rate )
    pub fn withdraw(&mut self, staking_amount: Uint128) -> Uint128 {
        println!(
            "\n[withdraw] user withdrawing {} staking tokens...",
            staking_amount
        );
        println!("[withdraw] old state: {:?}", self);

        let st_dec = Decimal::from_ratio(staking_amount, Uint128::one());
        let underlying_dec = st_dec * self.exchange_rate;
        let underlying = underlying_dec.to_uint_floor();

        // Update state
        self.total_supply = self.total_supply.saturating_sub(staking_amount);
        self.total_bond_amount = self.total_bond_amount.saturating_sub(underlying);

        self.update_exchange_rate();

        println!("[withdraw] underlying redeemed: {}", underlying);
        println!("[withdraw] new state: {:?}", self);

        underlying
    }

    /// Simulate time passing, applying the annual APR for a fraction of a year.
    /// e.g., if time_factor=1.0 => a whole year => 10% reward, minus 5% commission on that reward => net 9.5%ish
    ///
    /// formula: net_reward = total_bond_amount * annual_apr * time_factor
    /// commission = net_reward * validator_commission
    /// final_reward = net_reward - commission
    /// total_bond_amount += final_reward
    pub fn apply_apr(&mut self, time_factor: Decimal) {
        println!(
            "\n[apply_apr] time_factor={} (1.0 means 1 year)",
            time_factor
        );
        println!("[apply_apr] old state: {:?}", self);

        // 1) gross_reward = total_bond_amount * annual_apr * time_factor
        let bond_amount_dec = Decimal::from_ratio(self.total_bond_amount, Uint128::one());
        let gross_reward = bond_amount_dec * self.annual_apr * time_factor;

        // 2) commission = gross_reward * validator_commission
        let commission = gross_reward * self.validator_commission;

        // 3) net_reward = gross_reward - commission
        let net_reward = gross_reward - commission;

        // Convert net_reward to integer
        let net_reward_int = net_reward.to_uint_floor();

        // 4) total_bond_amount += net_reward_int
        self.total_bond_amount += net_reward_int;

        // 5) update exchange rate
        self.update_exchange_rate();

        println!(
            "[apply_apr] gross_reward={} commission={} net_reward={} => net_reward_int={}",
            gross_reward, commission, net_reward, net_reward_int
        );
        println!("[apply_apr] new state: {:?}", self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Uint128;

    #[test]
    fn test_apr_flow() {
        // Start a new state: 10% APR, 5% commission
        let mut state = State::new();
        // By default:
        // annual_apr = 0.10
        // validator_commission = 0.05

        // 1) user deposits 100 underlying
        let minted1 = state.deposit(Uint128::new(100));
        // => minted ~100, exchange_rate=1.0
        assert_eq!(minted1, Uint128::new(100));

        // 2) user deposits another 100
        // => minted ~100, total_bond_amount=200
        let minted2 = state.deposit(Uint128::new(100));
        assert_eq!(minted2, Uint128::new(100));
        // total_bond_amount=200, total_supply=200 => exchange_rate=1.0

        // 3) simulate 1 year passing
        // => 10% reward on 200=20, commission=1 => net=19
        state.apply_apr(Decimal::one()); // time_factor=1.0 => 1 year
        // total_bond_amount=219, total_supply=200 => exchange_rate=1.095
        // (since 219 / 200 = 1.095)

        // 4) user withdraws 100 staking tokens
        // => user should get floor(100 * 1.095)=109 underlying
        let redeemed = state.withdraw(Uint128::new(100));
        assert_eq!(redeemed, Uint128::new(109));
        // left in the pool: total_bond_amount=110, total_supply=100 => rate=1.1

        // 5) simulate half a year passing => time_factor=0.5
        // gross_reward=110 * 0.10 * 0.5=5.5 => commission=5.5 * 0.05=0.275 => net=5.225 => floor=5
        state.apply_apr(Decimal::from_ratio(1u128, 2u128)); // 0.5
        // => total_bond_amount=115 => total_supply=100 => exchange_rate=1.15
        assert_eq!(state.total_bond_amount, Uint128::new(115));

        // 6) final withdraw of 100 staking tokens => ~ 115
        let final_redeem = state.withdraw(Uint128::new(100));
        // => user gets 115
        assert_eq!(final_redeem, Uint128::new(115));
        // => total_bond_amount=0, total_supply=0 => exchange_rate=1.0

        println!("\nAll done, final state: {:?}", state);
    }
}

fn main() {
    println!("Run: cargo test -- --nocapture --test-threads=1");
}
