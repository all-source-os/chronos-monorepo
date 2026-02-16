package clients

import (
	"context"
	"fmt"
	"net/smtp"
	"os"
	"strconv"
)

// EmailClient defines methods for sending emails.
type EmailClient interface {
	SendEmail(ctx context.Context, req SendEmailRequest) error
}

// SendEmailRequest contains parameters for sending an email.
type SendEmailRequest struct {
	To      string
	Subject string
	Body    string // plain text body
}

// SMTPConfig holds SMTP connection configuration.
type SMTPConfig struct {
	Host     string
	Port     int
	Username string
	Password string
	From     string
}

type smtpEmailClient struct {
	config SMTPConfig
}

// NewEmailClientFromEnv creates an EmailClient from environment variables:
//   - SMTP_HOST (required)
//   - SMTP_PORT (default 587)
//   - SMTP_USERNAME (required)
//   - SMTP_PASSWORD (required)
//   - SMTP_FROM (required, sender address)
func NewEmailClientFromEnv() (EmailClient, error) {
	host := os.Getenv("SMTP_HOST")
	if host == "" {
		return nil, fmt.Errorf("SMTP_HOST is required")
	}

	portStr := os.Getenv("SMTP_PORT")
	port := 587
	if portStr != "" {
		var err error
		port, err = strconv.Atoi(portStr)
		if err != nil {
			return nil, fmt.Errorf("invalid SMTP_PORT: %w", err)
		}
	}

	username := os.Getenv("SMTP_USERNAME")
	if username == "" {
		return nil, fmt.Errorf("SMTP_USERNAME is required")
	}

	password := os.Getenv("SMTP_PASSWORD")
	if password == "" {
		return nil, fmt.Errorf("SMTP_PASSWORD is required")
	}

	from := os.Getenv("SMTP_FROM")
	if from == "" {
		return nil, fmt.Errorf("SMTP_FROM is required")
	}

	return &smtpEmailClient{
		config: SMTPConfig{
			Host:     host,
			Port:     port,
			Username: username,
			Password: password,
			From:     from,
		},
	}, nil
}

// NewEmailClient creates an EmailClient with the given config.
func NewEmailClient(config SMTPConfig) EmailClient {
	return &smtpEmailClient{config: config}
}

func (c *smtpEmailClient) SendEmail(_ context.Context, req SendEmailRequest) error {
	addr := fmt.Sprintf("%s:%d", c.config.Host, c.config.Port)
	auth := smtp.PlainAuth("", c.config.Username, c.config.Password, c.config.Host)

	msg := fmt.Sprintf("From: %s\r\nTo: %s\r\nSubject: %s\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n%s",
		c.config.From, req.To, req.Subject, req.Body)

	return smtp.SendMail(addr, auth, c.config.From, []string{req.To}, []byte(msg))
}
