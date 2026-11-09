use crate::models::{ForwardingConfig, SmtpConfig};
use lettre::message::header::{ContentType, HeaderName, HeaderValue};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use mail_parser::{MessageParser, MimeHeaders};
use std::time::Duration;
use tracing::info;

pub struct ForwardResult {
    pub message_id: String,
    pub original_sender: String,
    pub subject: String,
    pub forwarded_to: String,
    pub success: bool,
    pub error: Option<String>,
}

pub struct Forwarder;

impl Forwarder {
    pub async fn forward_email(
        raw_bytes: &[u8],
        forward_cfg: &ForwardingConfig,
        smtp_cfg: &SmtpConfig,
    ) -> ForwardResult {
        let parsed = match MessageParser::default().parse(raw_bytes) {
            Some(msg) => msg,
            None => {
                return ForwardResult {
                    message_id: "unknown".to_string(),
                    original_sender: "unknown".to_string(),
                    subject: "Corrupted Email".to_string(),
                    forwarded_to: forward_cfg.target_emails.join(", "),
                    success: false,
                    error: Some("Failed to parse raw RFC822 email payload".to_string()),
                };
            }
        };

        let message_id = parsed
            .message_id()
            .unwrap_or("unknown-id")
            .to_string();

        let (sender_name, sender_addr) = parsed
            .from()
            .and_then(|f| f.first())
            .map(|addr| {
                let name = addr.name.as_deref().unwrap_or("").to_string();
                let email = addr
                    .address
                    .as_deref()
                    .unwrap_or("unknown@sender.com")
                    .to_string();
                (name, email)
            })
            .unwrap_or_else(|| (String::new(), "unknown@sender.com".to_string()));

        let original_sender_display = if sender_name.is_empty() {
            sender_addr.clone()
        } else {
            format!("{} <{}>", sender_name, sender_addr)
        };

        let original_subject = parsed.subject().unwrap_or("(No Subject)");
        let original_to = parsed
            .to()
            .map(|to_addr| match to_addr {
                mail_parser::Address::List(list) => list
                    .iter()
                    .filter_map(|a| a.address.as_deref())
                    .collect::<Vec<_>>()
                    .join(", "),
                mail_parser::Address::Group(groups) => groups
                    .iter()
                    .flat_map(|g| g.addresses.iter())
                    .filter_map(|a| a.address.as_deref())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
            .unwrap_or_default();

        let original_date = parsed
            .date()
            .map(|d| {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                    d.year, d.month, d.day, d.hour, d.minute, d.second
                )
            })
            .unwrap_or_else(|| "Unknown date".to_string());

        // Prepare Forward Subject
        let final_subject = if forward_cfg.subject_prefix.is_empty() {
            original_subject.to_string()
        } else {
            format!("{} {}", forward_cfg.subject_prefix.trim(), original_subject)
        };

        // Prepare Banner
        let text_banner = if forward_cfg.include_banner {
            format!(
                "---------- Forwarded message ---------\n\
                 From: {}\n\
                 Date: {}\n\
                 Subject: {}\n\
                 To: {}\n\
                 --------------------------------------\n\n",
                original_sender_display, original_date, original_subject, original_to
            )
        } else {
            String::new()
        };

        let html_banner = if forward_cfg.include_banner {
            format!(
                "<div style=\"border-left: 3px solid #2563eb; padding-left: 12px; margin-bottom: 16px; color: #475569; font-size: 13px; font-family: sans-serif;\">\
                 <strong>---------- Forwarded message ---------</strong><br>\
                 <strong>From:</strong> {}<br>\
                 <strong>Date:</strong> {}<br>\
                 <strong>Subject:</strong> {}<br>\
                 <strong>To:</strong> {}<br>\
                 --------------------------------------\
                 </div><br>",
                html_escape(&original_sender_display),
                html_escape(&original_date),
                html_escape(original_subject),
                html_escape(&original_to)
            )
        } else {
            String::new()
        };

        // Body Contents
        let text_cow = parsed.body_text(0);
        let original_text = text_cow.as_deref().unwrap_or("");
        let html_cow = parsed.body_html(0);
        let original_html = html_cow.as_deref().unwrap_or("");

        let combined_text = format!("{}{}", text_banner, original_text);
        let combined_html = if !original_html.is_empty() {
            Some(format!("{}{}", html_banner, original_html))
        } else {
            None
        };

        // Build Email Message
        let mut builder = Message::builder().subject(final_subject);

        // Sender Address (configured Relay sender)
        let from_mailbox: Mailbox = match format!("{} <{}>", smtp_cfg.from_name, smtp_cfg.from_email).parse() {
            Ok(mb) => mb,
            Err(_) => match smtp_cfg.from_email.parse() {
                Ok(mb) => mb,
                Err(e) => {
                    return ForwardResult {
                        message_id,
                        original_sender: original_sender_display,
                        subject: original_subject.to_string(),
                        forwarded_to: forward_cfg.target_emails.join(", "),
                        success: false,
                        error: Some(format!("Invalid from_email address: {}", e)),
                    };
                }
            },
        };
        builder = builder.from(from_mailbox);

        // Reply-To header set to original sender so replying in Gmail replies to sender
        if forward_cfg.preserve_reply_to {
            if let Ok(reply_to_mb) = sender_addr.parse::<Mailbox>() {
                builder = builder.reply_to(reply_to_mb);
            }
        }

        // Custom tracing headers
        let hdr_name = HeaderName::new_from_ascii_str("X-Forwarded-For");
        let hdr_val = HeaderValue::new(hdr_name, original_sender_display.clone());
        builder = builder.raw_header(hdr_val);

        // Destination Recipients (Target Gmail accounts)
        for target in &forward_cfg.target_emails {
            let trimmed = target.trim();
            if !trimmed.is_empty() {
                if let Ok(to_mb) = trimmed.parse::<Mailbox>() {
                    builder = builder.to(to_mb);
                }
            }
        }

        // Build MultiPart Body (Text, HTML, Attachments)
        let body_part = if let Some(html_content) = combined_html {
            MultiPart::alternative()
                .singlepart(SinglePart::plain(combined_text))
                .singlepart(SinglePart::html(html_content))
        } else {
            MultiPart::alternative().singlepart(SinglePart::plain(combined_text))
        };

        // Process Attachments
        let mut mixed = MultiPart::mixed().multipart(body_part);
        for attachment in parsed.attachments() {
            let filename = attachment
                .attachment_name()
                .unwrap_or("unnamed_attachment")
                .to_string();

            let content_type_str = attachment
                .content_type()
                .map(|ct| ct.c_type.as_ref())
                .unwrap_or("application/octet-stream");

            let ct: ContentType = content_type_str
                .parse()
                .unwrap_or(ContentType::parse("application/octet-stream").unwrap());

            let att = Attachment::new(filename).body(attachment.contents().to_vec(), ct);
            mixed = mixed.singlepart(att);
        }

        let message = match builder.multipart(mixed) {
            Ok(msg) => msg,
            Err(e) => {
                return ForwardResult {
                    message_id,
                    original_sender: original_sender_display,
                    subject: original_subject.to_string(),
                    forwarded_to: forward_cfg.target_emails.join(", "),
                    success: false,
                    error: Some(format!("Failed to build MIME message: {}", e)),
                };
            }
        };

        // Send via Exchange SMTP Relay
        match Self::deliver_smtp(message, smtp_cfg).await {
            Ok(_) => {
                info!(
                    "Successfully forwarded message '{}' to {:?}",
                    original_subject, forward_cfg.target_emails
                );
                ForwardResult {
                    message_id,
                    original_sender: original_sender_display,
                    subject: original_subject.to_string(),
                    forwarded_to: forward_cfg.target_emails.join(", "),
                    success: true,
                    error: None,
                }
            }
            Err(err_msg) => ForwardResult {
                message_id,
                original_sender: original_sender_display,
                subject: original_subject.to_string(),
                forwarded_to: forward_cfg.target_emails.join(", "),
                success: false,
                error: Some(err_msg),
            },
        }
    }

    pub async fn send_test_email(
        smtp_cfg: &SmtpConfig,
        target_emails: &[String],
    ) -> Result<String, String> {
        let recipient = target_emails
            .first()
            .cloned()
            .unwrap_or_else(|| "admin@example.com".to_string());

        let from_mb: Mailbox = format!("{} <{}>", smtp_cfg.from_name, smtp_cfg.from_email)
            .parse()
            .map_err(|e| format!("Invalid from address: {}", e))?;

        let to_mb: Mailbox = recipient
            .parse()
            .map_err(|e| format!("Invalid recipient address '{}': {}", recipient, e))?;

        let body_text = format!(
            "Hello,\n\nThis is a test email sent from Smart Email Forwarder through the Internal MS Exchange SMTP Relay (Host: {}:{}).\n\nIf you see this message in Gmail, outbound forwarding is working perfectly!\n\nTimestamp: {}",
            smtp_cfg.host,
            smtp_cfg.port,
            chrono::Utc::now().to_rfc3339()
        );

        let email = Message::builder()
            .from(from_mb)
            .to(to_mb)
            .subject("[Smart Email Forwarder] Test Verification Email")
            .body(body_text)
            .map_err(|e| format!("Message build error: {}", e))?;

        Self::deliver_smtp(email, smtp_cfg).await?;
        Ok(recipient)
    }

    async fn deliver_smtp(email: Message, cfg: &SmtpConfig) -> Result<(), String> {
        let mailer = Self::create_mailer(cfg)?;

        mailer
            .send(email)
            .await
            .map_err(|e| format!("Exchange SMTP Relay error: {}", e))?;

        Ok(())
    }

    fn create_mailer(cfg: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
        let security = cfg.security.to_lowercase();

        let mut transport_builder = match security.as_str() {
            "plain" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
                .port(cfg.port)
                .tls(Tls::None),
            "starttls" => {
                if cfg.allow_self_signed {
                    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
                        .port(cfg.port)
                } else {
                    AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
                        .map_err(|e| format!("STARTTLS setup error: {}", e))?
                        .port(cfg.port)
                }
            }
            "ssl" => {
                if cfg.allow_self_signed {
                    AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
                        .port(cfg.port)
                } else {
                    AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
                        .map_err(|e| format!("SSL setup error: {}", e))?
                        .port(cfg.port)
                }
            }
            _ => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
                .port(cfg.port)
                .tls(Tls::None),
        };

        transport_builder = transport_builder.timeout(Some(Duration::from_secs(30)));

        if cfg.auth_required {
            let user = cfg.username.trim();
            let pass = cfg.password.as_deref().unwrap_or("").trim();
            if user.is_empty() || pass.is_empty() {
                if cfg.use_exchange_credentials {
                    return Err("Authentication is configured to use Exchange mailbox credentials, but Exchange username or password is missing. Please configure Exchange Mailbox settings first.".to_string());
                }
                return Err("Authentication is enabled but username or password is empty".to_string());
            }
            transport_builder = transport_builder.credentials(Credentials::new(
                user.to_string(),
                pass.to_string(),
            ));
        }

        Ok(transport_builder.build())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
