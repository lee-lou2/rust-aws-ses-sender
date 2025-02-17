use crate::config;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use aws_sdk_sesv2::{config::Region, Client};

/// send_email
/// Send email using AWS SES
/// Environment variables AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION are required for sending
pub async fn send_email(
    sender: &str,
    recipient: &str,
    subject: &str,
    body: &str,
) -> Result<String, aws_sdk_sesv2::Error> {
    let envs = config::get_environments();
    let aws_region = &envs.aws_region;
    let region_provider = RegionProviderChain::first_try(Region::new(aws_region))
        .or_default_provider()
        .or_else(Region::new(aws_region));

    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let client = Client::new(&shared_config);
    let message = Message::builder()
        .subject(
            Content::builder()
                .data(subject)
                .charset("UTF-8") // Using UTF-8 encoding
                .build()
                .expect("Failed to build subject content"),
        )
        .body(
            Body::builder()
                .html(
                    // Convert to HTML format
                    Content::builder()
                        .data(body)
                        .charset("UTF-8")
                        .build()
                        .expect("Failed to build body content"),
                )
                .build(),
        )
        .build();

    // Email send request
    let resp = client
        .send_email()
        .from_email_address(sender)
        .destination(Destination::builder().to_addresses(recipient).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    Ok(resp.message_id().unwrap_or_default().to_string()) // Return MessageId
}
