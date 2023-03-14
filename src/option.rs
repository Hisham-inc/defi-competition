use scrypto::radix_engine_interface::time::*;
use scrypto::prelude::*;

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
        strike_price: Decimal,
        duration: i64, 
        mint_badge_address: ResourceAddress,
        cctoken_a_address: ResourceAddress,
        cctoken_b_address: ResourceAddress,
        bonded_token_address: ResourceAddress,
        token_a_vault: Vault,
        token_b_vault: Vault
    }

    /// strike rate should be in the amm function

    impl ConstantSumOption {
        pub fn instantiate_option(token_a_address: ResourceAddress, token_a_name: String, token_a_symbol: String, token_b_address: ResourceAddress,
        token_b_name: String, token_b_symbol: String, strike_price: Decimal, duration: i64) -> ConstantSumOptionComponent {
            
            assert!(token_a_address == token_b_address, "Pool cant have same token");

            let mint_badge_address: ResourceAddress= ResourceBuilder::new_fungible()
                .metadata("Name", "LP Mint Badge")
                .divisibility(DIVISIBILITY_NONE)
                .create_with_no_initial_supply();

            let cctoken_a_address: ResourceAddress = ResourceBuilder::new_fungible()
                .metadata("Name", token_a_name)
                .metadata("Symbol", token_a_symbol)
                .mintable(rule!(require(mint_badge_address)), LOCKED)
                .burnable(rule!(require(mint_badge_address)), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .create_with_no_initial_supply();

            let cctoken_b_address: ResourceAddress = ResourceBuilder::new_fungible()
                .metadata("Name", token_b_name)
                .metadata("Symbol", token_b_symbol)
                .mintable(rule!(require(mint_badge_address)), LOCKED)
                .burnable(rule!(require(mint_badge_address)), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .create_with_no_initial_supply();
            
            let bonded_token_address: ResourceAddress = ResourceBuilder::new_fungible()
                .metadata("Name", "Bond Token")
                .metadata("Symbol", "BT")
                .mintable(rule!(require(mint_badge_address)), LOCKED)
                .burnable(rule!(require(mint_badge_address)), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .create_with_no_initial_supply();

             
            if Clock::current_time_is_at_or_before(Instant::new(duration), TimePrecision::Minute) == true 
            {panic!("Maturity of the pool is over")}; 

            let option_implementation: ConstantSumOptionComponent = Self {
                strike_price: strike_price,
                duration: duration,
                mint_badge_address,
                cctoken_a_address,
                cctoken_b_address,
                bonded_token_address,
                token_a_vault: Vault::new(token_a_address),
                token_b_vault: Vault::new(token_b_address),
            }
            .instantiate();

            return option_implementation
            
        }

         pub fn new_lend_user(&self) -> Bucket {
            ResourceBuilder::new_fungible()
                .metadata("Name", "User Badge of lending")
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(1)
        }

        pub fn option_a(&mut self, lock_token: Bucket) -> (Bucket, Bucket, Bucket) {
            assert!(lock_token.resource_address() == self.token_a_vault.resource_address(), "Wrong token provided");
            let lock_token_amount = lock_token.amount();
            let mint_badge: Bucket = borrow_resource_manager!(self.mint_badge_address).mint(1);

            self.token_a_vault.put(lock_token);

            let cctoken_a: Bucket = borrow_resource_manager!(self.cctoken_a_address).mint(lock_token_amount);
            let bonded_token_a: Bucket = borrow_resource_manager!(self.bonded_token_address).mint(lock_token_amount);
            return (mint_badge, cctoken_a, bonded_token_a);
        }

        pub fn option_b(&mut self, lock_token: Bucket) -> (Bucket, Bucket, Bucket) {
            assert!(lock_token.resource_address() == self.token_b_vault.resource_address(), "Wrong token provided");
            let lock_token_amount = lock_token.amount();
            let mint_badge: Bucket = borrow_resource_manager!(self.mint_badge_address).mint(1);

            self.token_b_vault.put(lock_token);

            let cctoken_b: Bucket = borrow_resource_manager!(self.cctoken_b_address).mint(lock_token_amount);
            let bonded_token_b: Bucket = borrow_resource_manager!(self.bonded_token_address).mint(lock_token_amount);
            return (mint_badge, cctoken_b, bonded_token_b);
        }                
    }
}
