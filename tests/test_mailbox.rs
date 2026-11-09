use smart_email_forwarder::models::{
    MailboxAttachmentInfo, MailboxListResponse, MailboxMessageDetail, MailboxMessageSummary,
};
use mail_parser::{MessageParser, MimeHeaders};

#[test]
fn test_mailbox_summary_and_detail_parsing() {
    let raw_email = b"From: Alice Smith <alice@example.com>\r\n\
To: Bob Jones <bob@example.com>\r\n\
Cc: Carol White <carol@example.com>\r\n\
Subject: Project Kickoff Meeting Notes\r\n\
Date: Mon, 09 Nov 2026 10:00:00 +0000\r\n\
Message-ID: <msg-12345@example.com>\r\n\
Content-Type: multipart/alternative; boundary=\"alt-boundary\"\r\n\
\r\n\
--alt-boundary\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hi Bob,\r\n\
Please review the attached notes.\r\n\
\r\n\
--alt-boundary\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>Hi Bob,</p><p>Please review the <strong>attached notes</strong>.</p>\r\n\
--alt-boundary--\r\n";

    let parsed = MessageParser::default().parse(raw_email).expect("Parse email");
    assert_eq!(parsed.subject().unwrap(), "Project Kickoff Meeting Notes");
    assert_eq!(parsed.message_id().unwrap(), "msg-12345@example.com");

    let text = parsed.body_text(0).unwrap();
    assert!(text.contains("Hi Bob"));

    let html = parsed.body_html(0).unwrap();
    assert!(html.contains("<strong>attached notes</strong>"));

    let summary = MailboxMessageSummary {
        uid: 101,
        message_id: parsed.message_id().unwrap().to_string(),
        subject: parsed.subject().unwrap().to_string(),
        from: "Alice Smith <alice@example.com>".to_string(),
        to: "Bob Jones <bob@example.com>".to_string(),
        date: "2026-11-09 10:00:00".to_string(),
        is_seen: false,
        is_forwarded: false,
        has_attachments: false,
        size_formatted: "1.2 KB".to_string(),
    };

    assert_eq!(summary.uid, 101);
    assert!(!summary.is_seen);
    assert!(!summary.is_forwarded);

    let detail = MailboxMessageDetail {
        uid: 101,
        message_id: summary.message_id.clone(),
        subject: summary.subject.clone(),
        from: summary.from.clone(),
        to: summary.to.clone(),
        cc: "Carol White <carol@example.com>".to_string(),
        date: summary.date.clone(),
        is_seen: false,
        is_forwarded: false,
        body_text: text.to_string(),
        body_html: Some(html.to_string()),
        raw_headers: "From: Alice Smith <alice@example.com>...".to_string(),
        attachments: vec![],
    };

    assert_eq!(detail.cc, "Carol White <carol@example.com>");
    assert!(detail.body_html.is_some());
}

#[test]
fn test_mailbox_attachments_extraction() {
    let raw_email = b"From: sender@example.com\r\n\
To: recipient@example.com\r\n\
Subject: Invoices\r\n\
Content-Type: multipart/mixed; boundary=\"mixed-boundary\"\r\n\
\r\n\
--mixed-boundary\r\n\
Content-Type: text/plain\r\n\
\r\n\
Please see invoices attached.\r\n\
\r\n\
--mixed-boundary\r\n\
Content-Type: application/pdf\r\n\
Content-Disposition: attachment; filename=\"invoice_nov2026.pdf\"\r\n\
\r\n\
%PDF-1.4 Mock PDF Content 123456\r\n\
--mixed-boundary\r\n\
Content-Type: image/png\r\n\
Content-Disposition: attachment; filename=\"receipt.png\"\r\n\
\r\n\
Mock PNG Content\r\n\
--mixed-boundary--\r\n";

    let parsed = MessageParser::default().parse(raw_email).expect("Parse email");
    let mut attachments = Vec::new();

    for (index, att) in parsed.attachments().enumerate() {
        let filename = att.attachment_name().unwrap_or("unnamed").to_string();
        let content_type = att
            .content_type()
            .map(|c| {
                if let Some(sub) = c.subtype() {
                    format!("{}/{}", c.c_type, sub)
                } else {
                    c.c_type.to_string()
                }
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let size_bytes = att.contents().len();

        attachments.push(MailboxAttachmentInfo {
            index,
            filename,
            content_type,
            size_bytes,
            size_formatted: format!("{} bytes", size_bytes),
        });
    }

    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].filename, "invoice_nov2026.pdf");
    assert_eq!(attachments[0].content_type, "application/pdf");
    assert!(attachments[0].size_bytes > 0);

    assert_eq!(attachments[1].filename, "receipt.png");
    assert_eq!(attachments[1].content_type, "image/png");

    let first_att = parsed.attachments().nth(0).unwrap();
    assert_eq!(first_att.contents(), b"%PDF-1.4 Mock PDF Content 123456");
}

#[test]
fn test_mailbox_pagination_math() {
    let total = 45;
    let limit = 20;
    let page = 1;
    let total_pages = (total + limit - 1) / limit;
    assert_eq!(total_pages, 3);

    let resp = MailboxListResponse {
        folder: "INBOX".to_string(),
        total,
        page,
        limit,
        total_pages,
        messages: vec![],
    };

    assert_eq!(resp.total_pages, 3);
    assert_eq!(resp.folder, "INBOX");
}
