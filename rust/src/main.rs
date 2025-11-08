use anyhow::Result;
use axum::{
    response::Redirect,
    routing::{get, get_service},
    Router,
};
use std::{path::PathBuf, sync::Arc};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

use claude_relay::routes::{
    create_admin_routes, create_api_router, create_gemini_router, create_openai_router,
    health_check, ping, ApiState, AppState, GeminiState, OpenAIState,
};
use claude_relay::services::{
    bedrock_relay::BedrockRelayService, claude_relay::ClaudeRelayConfig,
    gemini_relay::GeminiRelayService, pricing_service::PricingService, AccountScheduler,
    AdminService, ApiKeyService, ClaudeAccountService, ClaudeRelayService, UnifiedClaudeScheduler,
    UnifiedGeminiScheduler, UnifiedOpenAIScheduler,
};
use claude_relay::utils::{init_logger, HttpClient};
use claude_relay::{RedisPool, Settings};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file from project root (../.env) or current directory (.env)
    dotenvy::from_path("../.env")
        .or_else(|_| dotenvy::from_path(".env"))
        .ok(); // Ignore error if .env doesn't exist, rely on environment variables

    // Load configuration first (needed for logger initialization)
    let settings = Settings::new()?;

    // Initialize logging system
    init_logger(&settings)?;

    info!("🚀 Claude Relay Service (Rust) starting...");
    info!("📋 Configuration loaded");

    // Validate configuration
    if let Err(e) = settings.validate() {
        error!("❌ Configuration validation failed: {}", e);
        return Err(anyhow::anyhow!("Invalid configuration: {}", e));
    }
    info!("✅ Configuration validated");

    // Initialize Redis connection pool
    let redis = RedisPool::new(&settings)?;
    info!("🔌 Redis connection pool created");

    // Test Redis connection
    match redis.ping().await {
        Ok(_) => info!("✅ Redis connection established"),
        Err(e) => {
            error!("❌ Redis connection failed: {}", e);
            return Err(anyhow::anyhow!("Failed to connect to Redis: {}", e));
        }
    }

    // Initialize HTTP client
    let http_client = HttpClient::new(&settings)?;
    info!("🌐 HTTP client initialized");

    // Initialize services
    let settings_arc = Arc::new(settings.clone());
    let redis_arc = Arc::new(redis.clone());

    let account_service = Arc::new(
        ClaudeAccountService::new(redis_arc.clone(), settings_arc.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create account service: {}", e))?,
    );
    info!("👤 Account service initialized");

    let api_key_service = Arc::new(ApiKeyService::new(
        (*redis_arc).clone(),
        (*settings_arc).clone(),
    ));
    info!("🔑 API Key service initialized");

    let scheduler = Arc::new(AccountScheduler::new(
        redis_arc.clone(),
        account_service.clone(),
    ));
    info!("📅 Account scheduler initialized");

    // Initialize unified schedulers
    let unified_claude_scheduler = Arc::new(UnifiedClaudeScheduler::new(
        account_service.clone(),
        scheduler.clone(),
        redis_arc.clone(),
    ));
    info!("🎯 Unified Claude scheduler initialized");

    let unified_gemini_scheduler = Arc::new(UnifiedGeminiScheduler::new(
        account_service.clone(),
        scheduler.clone(),
        redis_arc.clone(),
        None, // sticky_session_ttl_hours: use default (1 hour)
    ));
    info!("🎯 Unified Gemini scheduler initialized");

    let unified_openai_scheduler = Arc::new(UnifiedOpenAIScheduler::new(
        account_service.clone(),
        scheduler.clone(),
        redis_arc.clone(),
        None, // sticky_session_ttl_hours: use default (1 hour)
    ));
    info!("🎯 Unified OpenAI scheduler initialized");

    // Create relay config (use default)
    let relay_config = ClaudeRelayConfig::default();

    // Get the underlying reqwest Client for ClaudeRelayService
    let reqwest_client = Arc::new(http_client.client().clone());

    let relay_service = Arc::new(ClaudeRelayService::new(
        relay_config,
        reqwest_client.clone(),
        redis_arc.clone(),
        account_service.clone(),
        scheduler.clone(),
    ));
    info!("🔄 Claude relay service initialized");

    // Create Gemini relay service
    let gemini_config = claude_relay::services::gemini_relay::GeminiRelayConfig::default();
    let gemini_service = Arc::new(GeminiRelayService::new(
        gemini_config,
        reqwest_client.clone(),
        redis_arc.clone(),
        account_service.clone(),
        scheduler.clone(),
    ));
    info!("🔄 Gemini relay service initialized");

    // Create Bedrock relay service
    let bedrock_config = claude_relay::services::bedrock_relay::BedrockRelayConfig::default();
    let bedrock_service = Arc::new(BedrockRelayService::new(
        bedrock_config,
        reqwest_client.clone(),
        redis_arc.clone(),
        account_service.clone(),
        scheduler.clone(),
    ));
    info!("🔄 Bedrock relay service initialized");

    // Create pricing service
    let pricing_service = Arc::new(PricingService::new(reqwest_client.clone()));
    info!("💰 Pricing service initialized");

    // Create admin service
    let jwt_secret = &settings.security.jwt_secret;
    if jwt_secret.is_empty() {
        error!("❌ JWT_SECRET is required for admin authentication");
        return Err(anyhow::anyhow!(
            "JWT_SECRET must be set (CRS_SECURITY__JWT_SECRET environment variable)"
        ));
    }
    let admin_service = Arc::new(AdminService::new(
        Arc::new(redis.clone()),
        jwt_secret.clone(),
    ));
    info!("👮 Admin service initialized");

    // Initialize admin from data/init.json (if exists)
    if let Err(e) = admin_service.initialize_admin_from_file().await {
        error!("⚠️  Failed to initialize admin from file: {}", e);
        info!("💡 Run setup to create admin credentials");
    }

    // Create shared application states
    let health_state = Arc::new(AppState {
        redis: redis.clone(),
    });

    let api_state = ApiState {
        redis: redis_arc.clone(),
        settings: settings_arc.clone(),
        account_service: account_service.clone(),
        api_key_service: api_key_service.clone(),
        scheduler: scheduler.clone(),
        relay_service,
        bedrock_service,
        unified_claude_scheduler,
        pricing_service: pricing_service.clone(),
    };

    let gemini_state = GeminiState {
        redis: redis_arc.clone(),
        settings: settings_arc.clone(),
        account_service: account_service.clone(),
        api_key_service: api_key_service.clone(),
        scheduler: scheduler.clone(),
        gemini_service,
        unified_gemini_scheduler,
        pricing_service: pricing_service.clone(),
    };

    let openai_state = OpenAIState {
        redis: redis_arc,
        settings: settings_arc,
        account_service,
        api_key_service: api_key_service.clone(),
        scheduler,
        unified_openai_scheduler,
    };

    // Setup static file serving for Vue SPA
    // Use absolute path based on CARGO_MANIFEST_DIR or fallback to relative path
    let static_dir = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir).parent().unwrap().join("web/admin-spa/dist")
    } else {
        // Fallback: try to detect from executable location
        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe_path.parent().and_then(|p| p.parent()).map(|p| p.join("web/admin-spa/dist"))
            .unwrap_or_else(|| PathBuf::from("../web/admin-spa/dist"))
    };

    // SPA fallback: serve index.html for all unmatched routes
    let index_path = static_dir.join("index.html");
    let serve_dir = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(&index_path));

    info!("📁 Static files serving from: {}", static_dir.display());

    // Verify static directory exists
    if !static_dir.exists() {
        error!("⚠️  Static directory does not exist: {}", static_dir.display());
        error!("⚠️  Frontend UI will not be available!");
    } else {
        info!("✅ Static directory verified: {}", static_dir.display());
    }

    // Prepare favicon file service
    let favicon_path = static_dir.join("favicon.ico");
    let serve_favicon = ServeFile::new(&favicon_path);

    // Build router
    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/admin-next") })) // Redirect root to admin
        .route("/favicon.ico", get_service(serve_favicon))
        .route("/health", get(health_check))
        .route("/ping", get(ping))
        .with_state(health_state)
        .nest(
            "/admin",
            create_admin_routes(admin_service.clone(), api_key_service.clone(), redis.clone()),
        )
        .nest(
            "/web",
            create_admin_routes(admin_service, api_key_service, redis.clone()),
        ) // For frontend compatibility
        .nest("/api", create_api_router(api_state.clone()))
        .nest("/claude", create_api_router(api_state))
        .nest("/gemini", create_gemini_router(gemini_state))
        .nest("/openai", create_openai_router(openai_state))
        .nest_service("/admin-next", serve_dir); // Serve Vue SPA

    // Get bind address
    let bind_addr = settings.bind_address();
    info!("✅ Server starting on http://{}", bind_addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", bind_addr, e))?;

    info!("🚀 Server ready on http://{}", bind_addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    info!("👋 Shutting down...");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}
