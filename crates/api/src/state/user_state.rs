use crate::config::database::Database;
use crate::config::parameter;
use crate::middleware::charging::ChargingManager;
use crate::repository::ai_model_repository::AiModelRepository;
use crate::repository::pricing_repository::PricingRepository;
use crate::repository::user_repository::UserRepository;
use crate::repository::wallet_repository::WalletRepository;
use crate::service::agent_analysis_service::AgentAnalysisService;
use crate::service::agent_service::AgentService;
use crate::service::campaign_service::CampaignService;
use crate::service::config_service::ConfigService;
use crate::service::crawler_service::CrawlerService;
use crate::service::dashboard_service::DashboardService;
use crate::service::email_verification_service::EmailVerificationService;
use crate::service::laozhang_client::LaoZhangClient;
use crate::service::platform_service::PlatformService;
use crate::service::promo_code_service::PromoCodeService;
use crate::service::referral_service::ReferralService;
use crate::service::social_account_service::SocialAccountService;
use crate::service::social_group_service::SocialGroupService;
use crate::service::template_service::TemplateService;
use crate::service::upload_task_service::UploadTaskService;
use crate::service::user_service::UserService;
use crate::service::video_service::VideoService;
use crate::service::wallet_service::WalletService;
use std::sync::Arc;

#[derive(Clone)]
pub struct UserState {
    pub db: Arc<Database>,
    pub user_service: UserService<UserRepository>,
    pub template_service: TemplateService,
    pub campaign_service: CampaignService,
    pub platform_service: PlatformService,
    pub config_service: ConfigService,
    pub wallet_service: WalletService,
    pub promo_code_service: PromoCodeService,
    pub email_verification_service: EmailVerificationService,
    pub social_account_service: SocialAccountService,
    pub social_group_service: SocialGroupService,
    pub referral_service: ReferralService,
    pub dashboard_service: DashboardService,
    pub crawler_service: CrawlerService,
    pub agent_service: AgentService,
    pub agent_analysis_service: AgentAnalysisService,
    pub video_service: VideoService,
    pub upload_task_service: UploadTaskService,
    pub charging_manager: ChargingManager,
}

impl UserState {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        let user_repo = UserRepository::new(db_conn.pool.clone());

        // Initialize LaoZhang client
        let laozhang_api_key = parameter::get("LAOZHANG_API_KEY");
        let laozhang_base_url = std::env::var("LAOZHANG_BASE_URL").ok();
        let laozhang_client = LaoZhangClient::new(laozhang_api_key, laozhang_base_url);

        // Initialize ChargingManager
        let pricing_repo = PricingRepository::new(db_conn.pool.clone());
        let ai_model_repo = AiModelRepository::new(db_conn.pool.clone());
        let wallet_repo = WalletRepository::new(db_conn.pool.clone());
        let charging_manager = ChargingManager::new(pricing_repo, ai_model_repo, wallet_repo);

        Self {
            db: db_conn.clone(),
            user_service: UserService::new(db_conn, user_repo),
            template_service: TemplateService::new(db_conn),
            campaign_service: CampaignService::new(db_conn),
            platform_service: PlatformService::new(db_conn),
            config_service: ConfigService::new(db_conn),
            wallet_service: WalletService::new(db_conn),
            promo_code_service: PromoCodeService::new(db_conn),
            email_verification_service: EmailVerificationService::new(db_conn),
            social_account_service: SocialAccountService::new(db_conn),
            social_group_service: SocialGroupService::new(db_conn),
            referral_service: ReferralService::new(db_conn.pool.clone()),
            dashboard_service: DashboardService::new(db_conn),
            crawler_service: CrawlerService::new(db_conn),
            agent_service: AgentService::new(db_conn.pool.clone()),
            agent_analysis_service: AgentAnalysisService::new(db_conn.pool.clone()),
            video_service: VideoService::new(
                db_conn.pool.clone(),
                WalletRepository::new(db_conn.pool.clone()),
                laozhang_client,
                ConfigService::new(db_conn),
            ),
            upload_task_service: UploadTaskService::new(db_conn),
            charging_manager,
        }
    }
}
