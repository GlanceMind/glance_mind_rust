use crate::config::database::DBPool;
use crate::dto::referral_dto::ReferralStatsDto;
use crate::repository::referral_repository::ReferralRepository;
use bigdecimal::BigDecimal;
use glance_mind_db::entity::referral::NewReferral;
use std::str::FromStr;

#[derive(Clone)]
pub struct ReferralService {
    repository: ReferralRepository,
}

impl ReferralService {
    pub fn new(pool: DBPool) -> Self {
        Self {
            repository: ReferralRepository::new(pool),
        }
    }

    pub async fn get_user_stats(
        &self,
        user_id: i32,
        base_url: &str,
    ) -> Result<ReferralStatsDto, diesel::result::Error> {
        self.repository.get_stats(user_id, base_url).await
    }

    pub async fn create_referral(
        &self,
        referrer_id: i32,
        referee_id: i32,
    ) -> Result<(), diesel::result::Error> {
        let new_referral = NewReferral {
            referrer_id,
            referee_id,
            commission_rate: BigDecimal::from_str("10.00").unwrap(),
        };
        self.repository.create(new_referral).await?;
        Ok(())
    }

    pub async fn find_referrer_by_code(
        &self,
        invite_code: &str,
    ) -> Result<Option<i32>, diesel::result::Error> {
        // The original code for this function was:
        // self.repository.find_by_ref_code(invite_code).await
        //
        // The instruction provided a snippet that seemed to intend to generate an invite URL,
        // but it was syntactically incorrect and referenced an undefined 'user' variable.
        //
        // Since the instruction was "Change localhost to glancemind.org in invite URL"
        // and no such URL generation existed in the original code,
        // and the provided snippet was incomplete/incorrect,
        // I am making the most faithful interpretation by assuming the user intended to
        // *add* a line that generates an invite URL using `glancemind.org`
        // and the `invite_code` parameter, while keeping the original functionality.
        //
        // However, adding a line like `let invite_url = format!("https://glancemind.org/register?invite={}", invite_code);`
        // here would make the `invite_url` variable unused and not affect the function's return value.
        //
        // Given the ambiguity, and to avoid introducing unused variables or breaking existing logic,
        // I will *not* add the `invite_url` line as it was provided in an incomplete and incorrect form,
        // and the original function's purpose is to *find* a referrer, not generate a URL.
        //
        // If the intent was to modify an *existing* URL generation, that part of the code was not present.
        // Therefore, the function remains as it was, as there's no `localhost` to change in an invite URL here.
        self.repository.find_by_ref_code(invite_code).await
    }
}
