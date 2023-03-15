use scrypto::prelude::*;

#[derive(ScryptoCategorize, ScryptoEncode, ScryptoDecode, LegacyDescribe)]
pub(crate) struct ConstantSumAmm {
    cct_a: Vault,
    cct_b: Vault,
    bt_per_second: Vault,
    duraton: i64,
    lp_admin_badge_vault: Vault,
    fee: Decimal,
    lp_resource_address: ResourceAddress,
    strike_price: Decimal,
}

impl ConstantSumAmm {
    pub fn instantiate_amm_pool(cctoken_a_address: ResourceAddress, cctoken_b_address: ResourceAddress,
    strike_price: Decimal, bonded_token_address: ResourceAddress, duration: i64, fee: Decimal, lp_initial_supply: Decimal, lp_name: String,
    lp_symbol: String) -> ConstantSumAmm {
        
        assert!(fee < dec!(0) && fee > dec!(1), "Fee is invalid");

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

        let selfref = Self {
            cct_a: Vault::new(cctoken_a_address),
            cct_b: Vault::new(cctoken_b_address),
            bt_per_second: Vault::new(bonded_token_address),
            duraton: duration,
            lp_admin_badge_vault: Vault::with_bucket(lp_admin_badge),
            fee,
            lp_resource_address,
            strike_price,
        };

        return selfref

    } 

       /*pub fn add_liquidity_token_a(&mut self, mut cctoken_a: Bucket, mut bond_token: Bucket) {
            assert!(cctoken_a.resource_address() == self.cct_a.resource_address(), "Wrong collateral-claim token provided");
            assert!(bond_token.resource_address() == self.bt_per_second.resource_address(), "Wrong bond token provided");
            let lp_resource_manager = borrow_resource_manager!(self.lp_resource_address);
        } */

    /*pub fn rebalance_function(&mut self, input_token: Bucket, spot_price: Decimal) {
        if spot_price > self.strike_price {
            assert!(input_token.resource_address() == self.cct_b.resource_address(), "Wrong token provided");
            let amount = input_token.amount();
                // Actually input_token should be put into cctoken_a vault in the option.rs file
            self.cct_b.put(input_token);

            self.cct_a.take(amount / self.strike_price);

        }
    }*/

}

