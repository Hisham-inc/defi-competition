use scrypto::radix_engine_interface::time::*;
use scrypto::prelude::*;

#[blueprint]
mod amm_implementation {
    struct ConstantSumAmm {
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
        // fees required for swapping tokens
        fee: Decimal,
        // resource address of LP token 
        lp_resource_address: ResourceAddress,
        //strike rate
        strike_rate: Decimal,
        //constant_product
        constant_product: Decimal,
        //LP token per collateral claim and bond tokens supplied
        lp_per_asset_ratio: Decimal,
        //amount of collateral claim token a supplied initially
        initial_cctoken_a: Decimal,
        //amount of collateral claim token a supplied initially
        initial_cctoken_b: Decimal,
        // amount of bonded token supplied initially
        initial_bonded_token_per_second: Decimal,
        // Apr
        annual_interest_rate: Decimal
    }

    impl ConstantSumAmm {
        // Instantiating liquiidity pool
        pub fn instantiate_liquidity(cctoken_a: Bucket, cctoken_b: Bucket, bonded_token_per_second: Bucket, duration: i64,
        strike_rate: Decimal, lp_initial_supply: Decimal, fee: Decimal, lp_name: String, lp_symbol: String) -> (ComponentAddress, Bucket) {    
           
            let lp_admin_badge: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "Liquidity Provider Admin badge")
                .metadata("Usage", "Needed to mint LP tokens")
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let initial_cctoken_a = cctoken_a.amount();
            let initial_cctoken_b = cctoken_b.amount();
            let initial_bonded_token_per_second = bonded_token_per_second.amount();
            
            let total_bonded_token: Decimal = initial_bonded_token_per_second * Decimal::from(duration);
            let annual_interest_rate = total_bonded_token / (initial_cctoken_a + initial_cctoken_b);
        
            let constant_product = (initial_cctoken_a + initial_cctoken_b) * initial_bonded_token_per_second;
            let sqrt_constant_product = constant_product.powi(1/2);

            let lp_resource_address = ResourceBuilder::new_fungible()
                .metadata("Name", lp_name)
                .metadata("Symbol", lp_symbol)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mintable(rule!(require(lp_admin_badge.resource_address())), LOCKED)
                .burnable(rule!(require(lp_admin_badge.resource_address())), LOCKED)
                .create_with_no_initial_supply();

            let lp_per_asset_ratio: Decimal = lp_initial_supply / ((initial_cctoken_a + initial_cctoken_b) * initial_bonded_token_per_second); 

            let lp_token: Bucket = lp_admin_badge.authorize(|| {
                borrow_resource_manager!(lp_resource_address).mint(sqrt_constant_product)
            });
                
            // Calculating the maturity of the pool
            assert!(Clock::current_time_is_at_or_before(Instant::new(duration), TimePrecision::Minute), "Maturity of the pool is over");
    
    
            let amm_implementation = Self {
                cct_a: Vault::with_bucket(cctoken_a),
                cct_b: Vault::with_bucket(cctoken_b),
                bt_per_second_vault: Vault::with_bucket(bonded_token_per_second),
                duration,
                lp_resource_address,
                lp_admin_badge_vault: Vault::with_bucket(lp_admin_badge),
                fee,
                strike_rate,
                constant_product,
                lp_per_asset_ratio,
                initial_cctoken_a,
                initial_cctoken_b,
                initial_bonded_token_per_second,
                annual_interest_rate
            }
            .instantiate()
            .globalize();
            
    
            return (amm_implementation, lp_token)
    
        }

        pub fn deposit_liquidity(&mut self, cctoken_a: Bucket, cctoken_b: Bucket, bonded_token_per_second: Bucket, 
        strike_price: Decimal, duration: i64) -> Bucket {
            assert!(duration == self.duration, "Maturity provided is wrong");

            assert!(strike_price == self.strike_rate, "Wrong strike rate");
        
            assert!(cctoken_a.resource_address() == self.cct_a.resource_address() && cctoken_b.resource_address() ==
            self.cct_b.resource_address(), "Wrong collateral-claim token provided");
            assert!(bonded_token_per_second.resource_address() == self.bt_per_second_vault.resource_address(), "Wrong bond token provided");

            let total_bonded_token: Decimal = bonded_token_per_second.amount() * Decimal::from(duration);

            let annual_interest_rate = total_bonded_token / (cctoken_a.amount() + cctoken_b.amount());
            assert!(annual_interest_rate == total_bonded_token / (cctoken_a.amount() + cctoken_b.amount()), "Ratio of tokens are wrong");
        
            let constant_product = self.constant_product + (cctoken_a.amount() + cctoken_b.amount()) * bonded_token_per_second.amount();
            let sqrt_constant_product = constant_product.powi(1/2);

            self.constant_product = constant_product;
            self.initial_cctoken_a += cctoken_a.amount();
            self.initial_cctoken_b += cctoken_b.amount();
            self.initial_bonded_token_per_second += bonded_token_per_second.amount();

            let lp_token: Bucket = self.lp_admin_badge_vault.authorize(|| {
                borrow_resource_manager!(self.lp_resource_address).mint(sqrt_constant_product)
            });

            self.cct_a.put(cctoken_a);
            self.cct_b.put(cctoken_b);
            self.bt_per_second_vault.put(bonded_token_per_second);

            return lp_token;
        }

        pub fn withdraw_liquidity(&mut self, lp_token: Bucket) -> (Bucket, Bucket, Bucket) {
            assert!(lp_token.resource_address() == self.lp_resource_address, "Wrong LP token provided");
                
            let x = self.initial_cctoken_a;
            let y = self.initial_cctoken_b;
            let z = self.initial_bonded_token_per_second;
            
            self.annual_interest_rate = (x + y) * z;
            

            let output_a = self.cct_a.take(x - (x * self.annual_interest_rate));
            let output_b = self.cct_b.take(y - (y * self.annual_interest_rate));
            let output_c = self.bt_per_second_vault.take(z - (z * self.annual_interest_rate));
            
            self.lp_admin_badge_vault.authorize(|| {
                lp_token.burn();
            });
            
            return (output_a, output_b, output_c)
        } 
    }
}
