use crate::config::database::DBPool;
use crate::dto::referral_dto::ReferralStatsDto;
use glance_mind_db::entity::referral::{NewReferral, Referral};
use glance_mind_db::entity::user::User;
use glance_mind_db::schema::{gm_referrals, gm_users};
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use std::str::FromStr;

#[derive(Clone)]
pub struct ReferralRepository {
    pool: DBPool,
}

impl ReferralRepository {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_referral: NewReferral) -> Result<Referral, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        diesel::insert_into(gm_referrals::table)
            .values(&new_referral)
            .get_result(&mut conn)
    }

    pub async fn find_by_referrer(&self, referrer_id: i32) -> Result<Vec<Referral>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");
        gm_referrals::table
            .filter(gm_referrals::referrer_id.eq(referrer_id))
            .load(&mut conn)
    }

    pub async fn get_stats(
        &self,
        user_id: i32,
        _base_url: &str,
    ) -> Result<ReferralStatsDto, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        // Get user to get invite_code
        let user: User = gm_users::table
            .find(user_id)
            .select(User::as_select())
            .first(&mut conn)?;

        let invite_code = match &user.invitation_code {
            Some(code) => code.clone(),
            None => return Err(DieselError::NotFound),
        };

        // Get referral counts
        let total_referrals: i64 = gm_referrals::table
            .filter(gm_referrals::referrer_id.eq(user_id))
            .count()
            .get_result(&mut conn)?;

        let active_referrals: i64 = gm_referrals::table
            .filter(gm_referrals::referrer_id.eq(user_id))
            .filter(gm_referrals::status.eq("ACTIVE"))
            .count()
            .get_result(&mut conn)?;

        // Get total earned (sum of all earnings)
        let total_earned: Option<BigDecimal> = gm_referrals::table
            .filter(gm_referrals::referrer_id.eq(user_id))
            .select(diesel::dsl::sum(gm_referrals::total_earned))
            .first(&mut conn)?;

        Ok(ReferralStatsDto {
            invite_code: invite_code.clone(),
            invite_url: format!("https://glancemind.org/register?invite={}", invite_code),
            total_referrals,
            active_referrals,
            total_earned: total_earned.unwrap_or(BigDecimal::from_str("0").unwrap()),
            commission_rate: BigDecimal::from_str("10.00").unwrap(),
        })
    }

    pub async fn find_by_ref_code(&self, ref_code: &str) -> Result<Option<i32>, DieselError> {
        let mut conn = self.pool.get().expect("Connection error");

        let user: Option<User> = gm_users::table
            .filter(gm_users::invitation_code.eq(ref_code))
            .select(User::as_select())
            .first(&mut conn)
            .optional()?;

        Ok(user.map(|u| u.id))
    }
}
