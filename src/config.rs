use dotenv::dotenv;
use once_cell::sync::Lazy;
use std::env;

/// Environment
/// Structure for environment variables
pub struct Environment {
    pub server_port: String,
    pub server_url: String,
    pub jwt_secret: String,
    pub aws_region: String,
    pub aws_ses_from_email: String,
    pub max_send_per_second: i32,
    pub sentry_dsn: String,
}

// Initialize and load the .env file only upon its first access using Lazy to create the Environment instance
static ENVIRONMENTS: Lazy<Environment> = Lazy::new(|| {
    // Load the .env file
    dotenv().ok();

    // Initialize the Environment struct with corresponding configuration values
    Environment {
        server_port: env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string()),
        server_url: env::var("SERVER_URL").unwrap_or_else(|_| "".to_string()),
        jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "".to_string()),
        aws_region: env::var("AWS_REGION").unwrap_or_else(|_| "ap-northeast-2".to_string()),
        aws_ses_from_email: env::var("AWS_SES_FROM_EMAIL").unwrap_or_else(|_| "".to_string()),
        max_send_per_second: env::var("MAX_SEND_PER_SECOND")
            .unwrap_or_else(|_| "24".to_string())
            .parse::<i32>()
            .unwrap_or(24),
        sentry_dsn: env::var("SENTRY_DSN").unwrap_or_else(|_| "".to_string()),
    }
});

/// get_environments
/// Returns the Environment structure instance
pub fn get_environments() -> &'static Environment {
    &ENVIRONMENTS
}
