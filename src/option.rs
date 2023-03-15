use crate::amm::ConstantSumAmm;
use scrypto::radix_engine_interface::time::*;
use scrypto::prelude::*;

#[blueprint]
mod option_implementation {
    struct ConstantSumOption {
        duration: i64, 
        mint_badge_vault: Vault,
        cctoken_a_address: ResourceAddress,
        cctoken_b_address: ResourceAddress,
        bonded_token_address: ResourceAddress,
        token_a_vault: Vault,
        token_b_vault: Vault,
        constant_sum_amm: ConstantSumAmm,
        strike_price: Decimal
    }

    /// strike rate should be in the amm function

    impl ConstantSumOption {
        pub fn instantiate_option(token_a_address: ResourceAddress, token_a_name: String, token_a_symbol: String, token_b_address: ResourceAddress,
        token_b_name: String, token_b_symbol: String, strike_price: Decimal, duration: i64) -> ComponentAddress {
            
            assert!(token_a_address != token_b_address, "Pool cant have same token");

            let mint_badge: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "LP Mint Badge")
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let mint_badge_rule = rule!(require(mint_badge.resource_address()));

            let cctoken_a_address: ResourceAddress = ResourceBuilder::new_fungible()
                .metadata("Name", token_a_name)
                .metadata("Symbol", token_a_symbol)
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .create_with_no_initial_supply();

            let cctoken_b_address: ResourceAddress = ResourceBuilder::new_fungible()
                .metadata("Name", token_b_name)
                .metadata("Symbol", token_b_symbol)
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .create_with_no_initial_supply();
            
            let bonded_token_address: ResourceAddress = ResourceBuilder::new_fungible()
                .metadata("Name", "Bond Token")
                .metadata("Symbol", "BT")
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .create_with_no_initial_supply();

             
            assert!(Clock::current_time_is_at_or_before(Instant::new(duration), TimePrecision::Minute), "Maturity of the pool is over");

            let constant_sum_amm: ConstantSumAmm = ConstantSumAmm::instantiate_amm_pool(
                cctoken_a_address, cctoken_b_address, strike_price, bonded_token_address,
                duration, dec!("0.5"), dec!(1000), "LPName".into(), "LPSym".into()); 

            let option_implementation: ConstantSumOptionComponent = Self {
                duration: duration,
                mint_badge_vault: Vault::with_bucket(mint_badge),
                cctoken_a_address,
                cctoken_b_address,
                bonded_token_address,
                token_a_vault: Vault::new(token_a_address),
                token_b_vault: Vault::new(token_b_address),
                strike_price,
                constant_sum_amm
            }
            .instantiate();

            option_implementation.globalize()
            
        }

         pub fn new_lend_user(&self) -> Bucket {
            ResourceBuilder::new_fungible()
                .metadata("Name", "User Badge of lending")
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(1)
        }

        pub fn option_a(&mut self, lock_token: Bucket) -> (Bucket, Bucket) {
            assert!(lock_token.resource_address() == self.token_a_vault.resource_address(), "Wrong token provided");
            let lock_token_amount = lock_token.amount();

            self.token_a_vault.put(lock_token);

            let cctoken_a: Bucket = self.mint_badge_vault.authorize(|| {
                borrow_resource_manager!(self.cctoken_a_address).mint(lock_token_amount)
            });
            let bonded_token_a: Bucket = self.mint_badge_vault.authorize(|| {
                borrow_resource_manager!(self.bonded_token_address).mint(lock_token_amount)
            });
            return (cctoken_a, bonded_token_a);
        }

        pub fn option_b(&mut self, lock_token: Bucket) -> (Bucket, Bucket) {
            assert!(lock_token.resource_address() == self.token_b_vault.resource_address(), "Wrong token provided");
            let lock_token_amount = lock_token.amount();

            self.token_b_vault.put(lock_token);

            let cctoken_b: Bucket = self.mint_badge_vault.authorize(|| {
                borrow_resource_manager!(self.cctoken_b_address).mint(lock_token_amount)
            });
            let bonded_token_b: Bucket = self.mint_badge_vault.authorize(|| {
                borrow_resource_manager!(self.bonded_token_address).mint(lock_token_amount)
            });
            return (cctoken_b, bonded_token_b);
        }                
    }
}
