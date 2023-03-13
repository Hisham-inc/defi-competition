use scrypto::prelude::*;

external_blueprint! {
    ConstantSumOption {
        fn instantiate_option(token_a_resource_address: ResourceAddress, token_b_resource_address: ResourceAddress,
            input_token: Bucket, token_name: String, token_symbol: String, strike_price: Decimal, duration: i64) -> (ConstantSumOptionComponent, Bucket, Bucket);
    }
}

external_component! {
    ConstantSumOptionComponent  {
        fn lend_tokens(&mut self, input_lend_token: Bucket, interest: Decimal) -> (Bucket, Bucket);
        fn borrow_tokens(&mut self, input_collateral: Bucket, cct_interest: Decimal) -> (Bucket, Bucket);
        fn show_balance(&self);
    }
}


#[blueprint]
mod constant_sum_amm {

    struct ConstantSumAmm {
        cct_a: Vault,
        cct_b: Vault,
        bt_per_second: Vault,
        duraton: i64,
        lp_admin_badge_vault: Vault,
        fee: Decimal,
        lp_resource_address: ResourceAddress,
    }

    impl ConstantSumAmm {
        pub fn instantiate_amm_pool(cctoken_a: Bucket, cctoken_b: Bucket, _strike_price: Decimal, bt_per_second: Bucket,
        duration: i64, fee: Decimal, lp_initial_supply: Decimal,lp_name: String, lp_symbol: String) -> (ConstantSumAmmComponent, Bucket){
            
          /*if strike_price >= dec!(1) {cctoken_b.amount() == cctoken_b.amount() / strike_price;}
            else {cctoken_a.amount() == strike_price * cctoken_a.amount();}*/

            assert!(fee < dec!(0) && fee > dec!(1), "Fee is invalid");
            
            let constant_product: Decimal = (cctoken_a.amount() + cctoken_b.amount()) * bt_per_second.amount();
            let _sqrt_constant_product: Decimal = constant_product.powi(1/2);

            // l is the marginal interest rate per second of bond token per total collateral claim tokens
            let _l: Decimal = bt_per_second.amount() / (cctoken_a.amount() + cctoken_b.amount());

            let lp_admin_badge: Bucket = ResourceBuilder::new_fungible()
                .metadata("Name", "Liquidity Provider Admin badge")
                .metadata("Usage", "Needed to mint LP tokens")
                .divisibility(DIVISIBILITY_NONE)
                .mint_initial_supply(1);

            let lp_resource_address = ResourceBuilder::new_fungible()
                .metadata("Name", lp_name)
                .metadata("Symbol", lp_symbol)
                .divisibility(DIVISIBILITY_MAXIMUM)
                .mintable(rule!(require(lp_admin_badge.resource_address())), LOCKED)
                .burnable(rule!(require(lp_admin_badge.resource_address())), LOCKED)
                .create_with_no_initial_supply();

            let lp_token = lp_admin_badge.authorize(|| {
                borrow_resource_manager!(lp_resource_address).mint(lp_initial_supply)
            });

            let constant_sum_amm: ConstantSumAmmComponent = Self {
                cct_a: Vault::with_bucket(cctoken_a),
                cct_b: Vault::with_bucket(cctoken_b),
                bt_per_second: Vault::with_bucket(bt_per_second),
                duraton: duration,
                lp_admin_badge_vault: Vault::with_bucket(lp_admin_badge),
                fee: fee,
                lp_resource_address,
            }
            .instantiate();

            (constant_sum_amm, lp_token)

        } 

        pub fn show_bal(&self) {
            self.cct_a.amount();
            self.cct_b.amount();
        }
    }
}