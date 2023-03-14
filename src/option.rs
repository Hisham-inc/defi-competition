use scrypto::radix_engine_interface::time::*;
use scrypto::prelude::*;

/// Trying to do a cross-blueprint call but dont know how to do 
external_blueprint! {
    ConstantSumAmm {
        fn instantiate_amm_pool(cc_token_a: Bucket, cc_token_b: Bucket, _strike_price: Decimal, bt_per_second: Bucket,
            duration: i64, fee: Decimal, lp_initial_supply: Decimal,lp_name: String, lp_symbol: String) -> (ConstantSumAmmComponent, Bucket);
    }
}

external_component! {
    ConstantSumAmmComponent {
        fn show_bal(&self);
    }
}

#[blueprint]
mod option_implementation {
    struct ConstantSumOption {
        /// Vault where input token is locked 
        lock_token_vault: Vault,
        strike_price: Decimal,
        mint_badge_vault: Vault,
        /// Resource address of bonded token
        bt_address: ResourceAddress,
        /// Vault where collateral claim tokens are stored
        cct_vault: Vault,
        /// Vault where bonded tokens are stored
        bt_vault: Vault,
        /// Duration of the maturity of the option
        duration: i64
    }

    impl ConstantSumOption {
        pub fn instantiate_option(token_a_resource_address: ResourceAddress, token_b_resource_address: ResourceAddress,
        input_token: Bucket, token_name: String, token_symbol: String, strike_price: Decimal, duration: i64)
        -> (ConstantSumOptionComponent, Bucket, Bucket) {
            
            assert!(!input_token.is_empty(), "No tokens have been deposited");

            assert!(token_a_resource_address == input_token.resource_address() ||
                    token_b_resource_address == input_token.resource_address(), "Wrong token provided");

            let mint_badge: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "Mint Badge")
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let cc_token: Bucket;

            let cctoken_a: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", token_name)
                .metadata("Symbol", token_symbol)
                .mintable(rule!(require(mint_badge.resource_address())), LOCKED)
                .burnable(rule!(require(mint_badge.resource_address())), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(strike_price * input_token.amount());

            let cctoken_b: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "token_name")
                .metadata("Symbol", "token_symbol")
                .mintable(rule!(require(mint_badge.resource_address())), LOCKED)
                .burnable(rule!(require(mint_badge.resource_address())), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(strike_price * input_token.amount());
            
            /// Checking whether the token deposited is token_a or token_b
            if input_token.resource_address() == token_a_resource_address {cc_token = cctoken_a;}
            else {cc_token = cctoken_b;}
            
            let bonded_token: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "Bond Token")
                .metadata("Symbol", "BT")
                .mintable(rule!(require(mint_badge.resource_address())), LOCKED)
                .burnable(rule!(require(mint_badge.resource_address())), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(input_token.amount() / strike_price);

            /// Setting the maturity of the options contract 
            assert!(Clock::current_time_is_at_or_before(Instant::new(duration), TimePrecision::Minute) == true, "Maturity of the pool is over");

            let option_implementation: ConstantSumOptionComponent = Self {
                lock_token_vault: Vault::with_bucket(input_token),
                strike_price: strike_price,
                mint_badge_vault: Vault::with_bucket(mint_badge),
                bt_address: bonded_token.resource_address(),
                cct_vault: Vault::new(cc_token.resource_address()),
                bt_vault: Vault::new(bonded_token.resource_address()),
                duration: duration,
            }
            .instantiate();
            (option_implementation, cc_token, bonded_token)
            
        }

        pub fn show_balance(&self) {
            self.lock_token_vault.amount();
        }

        /// method for lending token 
        pub fn lend_tokens(&mut self, input_lend_token: Bucket, bt_interest: Decimal) -> (Bucket, Bucket) {
            if input_lend_token.resource_address() == self.lock_token_vault.resource_address() {
                let bt_manager = borrow_resource_manager!(self.bt_address);
                let bt: Bucket = self.mint_badge_vault.authorize(|| bt_manager.mint(input_lend_token.amount()));
                let interest_bt: Bucket = self.mint_badge_vault.authorize(|| bt_manager.mint(bt_interest));

                return (bt, interest_bt)
            }
            else {panic!("The provided token doesnt have an Amm pool")}
        }

        /// method for borrowing tokens
        pub fn borrow_tokens(&mut self, input_collateral: Bucket, cct_interest: Decimal) -> (Bucket, Bucket) {
            if input_collateral.resource_address() == self.lock_token_vault.resource_address() {
                let cct_manager = borrow_resource_manager!(self.cct_vault.resource_address());
                let cct: Bucket = self.mint_badge_vault.authorize(|| cct_manager.mint(input_collateral.amount()));
                let interest_cct : Bucket = self.mint_badge_vault.authorize(|| cct_manager.mint(cct_interest));

                return (cct, interest_cct)
            }
            else {panic!("The provided token doesnt have an Amm pool")}
        }

        
    }
}
