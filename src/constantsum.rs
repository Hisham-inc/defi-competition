use scrypto::radix_engine_interface::time::*;
use scrypto::prelude::*;
  
  // importing a radiswap method that needs to be used in this blueprint
external_component! {
    RadiswapComponentTarget {
        // Imported method
        fn swap(&mut self, input_tokens: Bucket) -> Bucket;
    }
}

#[blueprint]
mod amm_implementation {
    struct ConstantSumAmm {
        // Vault for storing token_a
        token_a_vault: Vault,
        // Vault for storing token_b
        token_b_vault: Vault,
        // Vault for storing collateral claim token a
        cct_a: Vault,
        // Vault for storing collateral claim token a
        cct_b: Vault,
        // Vault for storing bonded_token per second
        bt_per_second_vault: Vault,
        // Maturity of the pool
        duration: i64,
        // Vault for storing LP admin badge
        lp_admin_badge_vault: Vault,
        // resource address of LP token 
        lp_resource_address: ResourceAddress,
        //strike rate of the pool [Check docs to understand strike rate]
        strike_rate: Decimal,
        //constant_product of the pool
        constant_product: Decimal,
        // Interest to be recieved from the liquidity pool
        interest: Decimal,
        // Component Address of Radiswap
        amm_address: ComponentAddress,
    }

    impl ConstantSumAmm {
        // Locking a token and minting collateral claim tokens and bond tokens
        pub fn locking_liquidity(token_a: Bucket, token_a_name: String, token_a_symbol: String, token_b: Bucket,
        token_b_name: String, token_b_symbol: String, duration: i64, required_interest: Decimal, strike_rate: Decimal, 
        lp_name: String, lp_symbol: String, amm_address: ComponentAddress) -> (ComponentAddress, Bucket, Bucket, Bucket) {   
            // Checking whether the ratio in which tokens are provided are correct 
            assert!(token_a.amount() / token_b.amount() == dec!(1) / strike_rate, "Tokens provided in the wrong ratio");

            // Admin badge used for doing privilaged actions like minting and burning cctokens, bonded tokens, LP tokens, etc
            let mint_badge: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "LP Mint Badge")
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);
            
            // Mint badge access rule
            let mint_badge_rule: AccessRule = rule!(require(mint_badge.resource_address()));

            // Collateral-claim-token of token_a
            let cctoken_a: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", token_a_name)
                .metadata("Symbol", token_a_symbol)
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(token_a.amount());
    
            // Collateral-claim-token of token_b
            let cctoken_b: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", token_b_name)
                .metadata("Symbol", token_b_symbol)
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(token_b.amount());
                
            // Bonded token    
            let bonded_token: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "Bond Token")
                .metadata("Symbol", "BT")
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mint_initial_supply(required_interest * (cctoken_a.amount() + (cctoken_b.amount() * strike_rate)));

            // Resource address of LP token
            let lp_resource_address = ResourceBuilder::new_fungible()
                .metadata("Name", lp_name)
                .metadata("Symbol", lp_symbol)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mintable(mint_badge_rule.clone(), LOCKED)
                .burnable(mint_badge_rule.clone(), LOCKED)
                .create_with_no_initial_supply();

            // Checking whether the minted collateral claim and bond tokens are in correct ratio
            assert!(bonded_token.amount() / cctoken_a.amount() + cctoken_b.amount() == required_interest, "Token ratios are wrong");

            // Interest to be recieved from liquidity pool
            let interest: Decimal = dec!(0);

            // Constant product of the AMM
            let constant_product: Decimal = dec!(0);
    
            // Maturity of the liquidity pool, after maturity the transaction will fail
            assert!(Clock::current_time_is_at_or_before(Instant::new(duration), TimePrecision::Minute), "Maturity of the pool is over");
    

            let amm_implementation = Self {
                token_a_vault: Vault::with_bucket(token_a),
                token_b_vault: Vault::with_bucket(token_b),
                cct_a: Vault::new(cctoken_a.resource_address()),
                cct_b: Vault::new(cctoken_b.resource_address()),
                bt_per_second_vault: Vault::new(bonded_token.resource_address()),
                duration,
                lp_resource_address,
                lp_admin_badge_vault: Vault::with_bucket(mint_badge),
                strike_rate,
                interest,
                amm_address,
                constant_product
            }
            .instantiate()
            .globalize();
            
            // Returning Component Address, collateral claim tokens and bonded tokens minted
            return (amm_implementation, cctoken_a, cctoken_b, bonded_token)
        }

        // Method to be called when spot_price is lesser than strike_rate, here you deposit cctoken_a and bonded_token   
        pub fn deposit_liquidity_a(&mut self, cctoken_a: Bucket, bonded_token: Bucket, strike_price: Decimal, duration: i64) -> Bucket {
            // Checking whether duration and strike rate provided are correct
            assert!(duration == self.duration, "Maturity provided is wrong");
            assert!(strike_price == self.strike_rate, "Wrong strike rate");
        
            // Checking whether collateral claim tokens and bond tokens provided are correct 
            assert!(cctoken_a.resource_address() == self.cct_a.resource_address() || bonded_token.resource_address() ==
            self.bt_per_second_vault.resource_address(), "Wrong collateral claim or bond token provided");

            // Checking whether collateral claim tokens and bond tokens provided are empty 
            assert!(!cctoken_a.is_empty() && !bonded_token.is_empty(), "Empty tokens provided");

            // Checking whether collateral claim tokens and bond tokens are provided in the correct ratio
            assert!(bonded_token.amount() / cctoken_a.amount() == self.interest, "Ratio of the tokens provided are wrong");

            // Setting up the interest rate
            self.interest += (bonded_token.amount() / Decimal::from(duration)) / cctoken_a.amount();

            // Adding current constant product to the actual constant product
            let delta_constant_product = cctoken_a.amount() * (bonded_token.amount() / Decimal::from(duration));
            self.constant_product += delta_constant_product;

            // Minting LP tokens
            let lp_token: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.lp_resource_address).mint(delta_constant_product.powi(1/2))
            });

            // Sending collateral claim tokens and bonded tokens to the liquidity pool
            self.cct_a.put(cctoken_a);
            self.bt_per_second_vault.put(bonded_token);

            // Returning LP tokens for the user to withdraw
            return lp_token;
        }

        // Method to be called when spot_price is greater than strike_rate, here you deposit cctoken_b and bonded_token
        pub fn deposit_liquidity_b(&mut self, cctoken_b: Bucket, bonded_token: Bucket, 
        strike_price: Decimal, duration: i64) -> Bucket {
            assert!(duration == self.duration, "Maturity provided is wrong");
            assert!(strike_price == self.strike_rate, "Wrong strike rate");
            
            assert!(cctoken_b.resource_address() == self.cct_b.resource_address() || bonded_token.resource_address() ==
            self.bt_per_second_vault.resource_address(), "Wrong collateral claim or bond token provided");
    
            assert!(!cctoken_b.is_empty() && !bonded_token.is_empty(), "Empty tokens provided");
    
            assert!(bonded_token.amount() / (cctoken_b.amount() / strike_price) == self.interest, "Ratio of the tokens provided are wrong");

            self.interest += (bonded_token.amount() / Decimal::from(duration)) / (cctoken_b.amount() / strike_price);
    
            let delta_constant_product = (cctoken_b.amount() / strike_price) * (bonded_token.amount() / Decimal::from(duration));
            self.constant_product += delta_constant_product;
    
             let lp_token: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.lp_resource_address).mint(delta_constant_product.powi(1/2))
            });
    
            self.cct_b.put(cctoken_b);
            self.bt_per_second_vault.put(bonded_token);
    
            return lp_token;
        }

         
        // This method to withdraw liquidity
        pub fn withdraw_liquidity(&mut self, lp_token: Bucket, strike_price: Decimal, duration: i64) -> (Bucket, Bucket, Bucket)  {
            assert!(!lp_token.is_empty(), "No LP tokens provided");
            assert!(lp_token.resource_address() == self.lp_resource_address, "Wrong LP token provided");

            assert!(duration == self.duration, "Maturity provided is wrong");
            assert!(strike_price == self.strike_rate, "Wrong strike rate");

            let lp_manager = borrow_resource_manager!(self.lp_resource_address);
            let share = lp_token.amount() / lp_manager.total_supply();

            self.constant_product = ((self.cct_a.amount() - (self.cct_a.amount() * share)) +
            ((self.cct_b.amount() / strike_price) - ((self.cct_b.amount() / strike_price) * share))) *
            ((self.bt_per_second_vault.amount() / Decimal::from(duration)) - (self.bt_per_second_vault.amount() / Decimal::from(duration)) * share);

            self.interest = ((self.bt_per_second_vault.amount() / Decimal::from(duration)) - (self.bt_per_second_vault.amount() / Decimal::from(duration)) * share) /
            (self.cct_a.amount() - (self.cct_a.amount() * share)) + ((self.cct_b.amount() / strike_price) - ((self.cct_b.amount() / strike_price) * share));

            self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.lp_resource_address).burn(lp_token)
            });

            (
                self.cct_a.take(self.cct_a.amount() * share),
                self.cct_b.take(self.cct_b.amount() * share),
                self.bt_per_second_vault.take(self.bt_per_second_vault.amount() * share)
            )
        }  

        
        pub fn rebalance_transaction(&mut self, collateral: Bucket) -> Bucket {
            assert!(!collateral.is_empty(), "No tokens provided");

            assert!(collateral.resource_address() == self.token_a_vault.resource_address() || 
            collateral.resource_address() == self.token_b_vault.resource_address(), "Wrong token provided");

            // When strike price is lesser than market price
            if collateral.resource_address() == self.token_b_vault.resource_address() {
                let withdraw = self.cct_a.take(collateral.amount() / self.strike_rate);

                let convert = self.convert_option(collateral, withdraw);

                let mut amm_component = RadiswapComponentTarget::at(self.amm_address);
                let token_b = amm_component.swap(convert.0);

                self.cct_b.put(convert.1);

                return token_b
            }
            // When strike price is bigger than market price
            else {
                let withdraw = self.cct_b.take(collateral.amount() * self.strike_rate);

                let convert = self.convert_option(collateral, withdraw);

                let mut amm_component = RadiswapComponentTarget::at(self.amm_address);
                let token_a = amm_component.swap(convert.0);

                self.cct_a.put(convert.1);

                return token_a
            }
        }

        pub fn convert_option(&mut self, lock_token: Bucket, cctoken: Bucket) -> (Bucket, Bucket) {
            assert!((lock_token.resource_address() == self.token_a_vault.resource_address() && cctoken.resource_address() ==
            self.cct_b.resource_address()) || (lock_token.resource_address() == self.token_b_vault.resource_address() && 
            cctoken.resource_address() == self.cct_a.resource_address()) , "Provided collateral or collateral claim token is wrong");

            // If collateral claim token is cctoken_a
            if lock_token.resource_address() == self.token_a_vault.resource_address() {
                let collateral_claim_token =  self.lp_admin_badge_vault.authorize(|| {
                    borrow_resource_manager!(self.cct_a.resource_address()).mint(lock_token.amount())
                });

                self.lp_admin_badge_vault.authorize(|| {
                    borrow_resource_manager!(self.cct_b.resource_address()).burn(cctoken)
                });

                let output_token = self.token_b_vault.take(lock_token.amount() * self.strike_rate);

                self.token_a_vault.put(lock_token);

                (output_token, collateral_claim_token)
            }
            // If collateral claim token is cctoken_a
            else {
                let collateral_claim_token = self.lp_admin_badge_vault.authorize(|| {
                    borrow_resource_manager!(self.cct_b.resource_address()).mint(lock_token.amount())
                });

                self.lp_admin_badge_vault.authorize(|| {
                    borrow_resource_manager!(self.cct_a.resource_address()).burn(cctoken)
                });

                let output_token = self.token_a_vault.take(lock_token.amount() / self.strike_rate);

                self.token_b_vault.put(lock_token);

                (output_token, collateral_claim_token)
            }
            
        }
        
        /*
        
            DRAFT-2 COMPLETED UPTO HERE 
        
         */

        pub fn option_a_deposit(&mut self, lock_token: Bucket) -> (Bucket, Bucket) {
            assert!(lock_token.resource_address() == self.token_a_vault.resource_address(), "Wrong token provided");

            let cctoken_a: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.cct_a.resource_address()).mint(lock_token.amount())
            });
            
            let bonded_token_a: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.bt_per_second_vault.resource_address()).mint(lock_token.amount())
            });

            self.token_a_vault.put(lock_token);

            return (cctoken_a, bonded_token_a);
        }

        pub fn option_a_withdraw(&mut self, unlock_token: ResourceAddress, cctoken_a: Bucket, bonded_token: Bucket) -> Bucket {
            assert!(cctoken_a.resource_address() == self.cct_a.resource_address() &&
            bonded_token.resource_address() == self.bt_per_second_vault.resource_address(), "Wrong tokens provided");
            assert!(unlock_token == self.token_a_vault.resource_address(), "Wrong token provided");
            
            let output_token: Bucket = self.token_a_vault.take(cctoken_a.amount());

            self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.cct_a.resource_address()).burn(cctoken_a)
            });

            self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.bt_per_second_vault.resource_address()).burn(bonded_token)
            });
            
            return output_token;
        }

        pub fn option_b_deposit(&mut self, lock_token: Bucket) -> (Bucket, Bucket) {
            assert!(lock_token.resource_address() == self.token_b_vault.resource_address(), "Wrong token provided");
            let lock_token_amount = lock_token.amount();

            self.token_b_vault.put(lock_token);

            let cctoken_b: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.cct_b.resource_address()).mint(lock_token_amount)
            });
            let bonded_token_b: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.bt_per_second_vault.resource_address()).mint(lock_token_amount / self.strike_rate)
            });

            return (cctoken_b, bonded_token_b);
        } 

        pub fn option_b_withdraw(&mut self, unlock_token: ResourceAddress, cctoken_b: Bucket, bonded_token: Bucket) -> Bucket {
            assert!(cctoken_b.resource_address() == self.cct_b.resource_address() &&
            bonded_token.resource_address() == self.bt_per_second_vault.resource_address(), "Wrong tokens provided");
            assert!(unlock_token == self.token_b_vault.resource_address(), "Wrong token provided");
            
            let output_token: Bucket = self.token_b_vault.take(cctoken_b.amount());

            self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.cct_b.resource_address()).burn(cctoken_b)
            });

            self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.bt_per_second_vault.resource_address()).burn(bonded_token)
            });
            
            return output_token;
        } 

        // When strike rate is lesser than market price
        pub fn lend_a(&mut self, lend_token: Bucket) -> (Bucket, Bucket) {
            assert!(lend_token.resource_address() == self.cct_b.resource_address(), "Swap the token");

            let returns = self.option_b_deposit(lend_token);

            let bond_token_per_second = (self.bt_per_second_vault.amount() / Decimal::from(self.duration)) -
            (self.constant_product / ((self.cct_b.amount() / self.strike_rate) + returns.1.amount()));
                                    // doubt^ 
            let bond_token = bond_token_per_second * Decimal::from(self.duration);

            self.cct_b.put(returns.0);
            let required_bond_token = self.bt_per_second_vault.take(bond_token);

            (returns.1, required_bond_token)
        }

        // When strike rate is bigger than market price
        pub fn lend_b(&mut self, lend_token: Bucket) -> (Bucket, Bucket) {
            assert!(lend_token.resource_address() == self.cct_a.resource_address(), "Swap the token");

            let returns = self.option_a_deposit(lend_token);

            let bond_token_per_second = (self.bt_per_second_vault.amount() / Decimal::from(self.duration)) -
            (self.constant_product / (self.cct_a.amount() + returns.1.amount()));

            let bond_token = bond_token_per_second * Decimal::from(self.duration);

            self.cct_a.put(returns.0);
            let required_bond_token = self.bt_per_second_vault.take(bond_token);
            
            (returns.1, required_bond_token)
        }
        
        pub fn borrow_a(&mut self, borrow_amount: Decimal, mut collateral: Bucket, cc_token: Bucket) -> (Bucket, Bucket, Bucket) {
            let y: Decimal = self.cct_b.amount() / self.strike_rate;
            let z: Decimal = self.bt_per_second_vault.amount() / Decimal::from(self.duration);
            let delta_y: Decimal = collateral.amount() / self.strike_rate;

            let bonded_token_per_second: Decimal = (self.constant_product / (y - delta_y)) - z;

            let bond_token: Decimal = bonded_token_per_second * Decimal::from(self.duration);

            let first_batch: Bucket = collateral.take(bond_token);
            let second_batch: Bucket = collateral.take(collateral.amount() - first_batch.amount());

            self.cct_b.take(borrow_amount);

            let returns = self.option_a_deposit(first_batch);
            self.bt_per_second_vault.put(returns.1);

            let convert = self.convert_option(second_batch, cc_token);

            (convert.0, convert.1, returns.0)
        }

        

    }

}