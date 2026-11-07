use mail_parser::{MessageParser, MimeHeaders};

#[test]
fn test_parse_simple_email() {
    let raw = b"From: John Doe <john@company.com>\r\n\
                To: recipient@company.com\r\n\
                Subject: Important Quarterly Review\r\n\
                Date: Thu, 7 Nov 2026 10:00:00 +0000\r\n\
                Message-ID: <test-1234@company.com>\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Hello, please review the attached document.";

    let parsed = MessageParser::default().parse(raw).expect("Failed to parse email");
    assert_eq!(parsed.subject().unwrap(), "Important Quarterly Review");
    assert_eq!(parsed.message_id().unwrap(), "test-1234@company.com");

    let from_addr = parsed.from().and_then(|f| f.first()).unwrap();
    assert_eq!(from_addr.name.as_deref().unwrap(), "John Doe");
    assert_eq!(from_addr.address.as_deref().unwrap(), "john@company.com");

    assert_eq!(parsed.body_text(0).as_deref().unwrap(), "Hello, please review the attached document.");
}

#[test]
fn test_parse_multipart_with_attachment() {
    let raw = b"From: sender@example.com\r\n\
                To: target@company.com\r\n\
                Subject: Email with PDF Attachment\r\n\
                Message-ID: <msg-attachment-99@company.com>\r\n\
                MIME-Version: 1.0\r\n\
                Content-Type: multipart/mixed; boundary=\"BOUNDARY123\"\r\n\
                \r\n\
                --BOUNDARY123\r\n\
                Content-Type: text/plain; charset=utf-8\r\n\
                \r\n\
                Plain text content.\r\n\
                --BOUNDARY123\r\n\
                Content-Type: application/pdf; name=\"invoice.pdf\"\r\n\
                Content-Disposition: attachment; filename=\"invoice.pdf\"\r\n\
                Content-Transfer-Encoding: base64\r\n\
                \r\n\
                JVBERi0xLjQKJcTl8uXrp/Og0MTGCjQgMCBvYmoKPDwKL1R5cGUgL1BhZ2VzCi9Db3VudCAxCj4+CmVuZG9iago=\r\n\
                --BOUNDARY123--";

    let parsed = MessageParser::default().parse(raw).expect("Failed to parse multipart");
    assert_eq!(parsed.subject().unwrap(), "Email with PDF Attachment");
    assert_eq!(parsed.attachment_count(), 1);

    let att = parsed.attachments().next().expect("No attachment found");
    assert_eq!(att.attachment_name().unwrap(), "invoice.pdf");
    assert!(!att.contents().is_empty());
}
