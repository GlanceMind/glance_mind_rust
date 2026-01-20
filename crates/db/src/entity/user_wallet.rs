use crate::schema::gm_user_wallets;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Queryable,
    Selectable,
    Insertable,
    AsChangeset,
    Identifiable,
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
#[diesel(primary_key(user_id))]
#[diesel(table_name = gm_user_wallets)]
pub struct UserWallet {
    pub user_id: i32,
    pub balance_points: BigDecimal,
    pub frozen_points: BigDecimal,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub deposit_cny: BigDecimal,
    pub deposit_usd: BigDecimal,
}

#[derive(Insertable)]
#[diesel(table_name = gm_user_wallets)]
pub struct NewUserWallet {
    pub user_id: i32,
    pub balance_points: Option<BigDecimal>,
    pub frozen_points: Option<BigDecimal>,
    pub deposit_cny: Option<BigDecimal>,
    pub deposit_usd: Option<BigDecimal>,
}
