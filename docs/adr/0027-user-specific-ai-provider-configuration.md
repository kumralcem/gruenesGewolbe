# User-Specific AI Provider Configuration

AI provider configuration should be user-specific app state rather than vault state. API keys, credentials, and preferred provider settings should live in app configuration or the OS keychain, while AI outputs and compact metadata provenance may be written into item records; this keeps copied vaults portable without leaking secrets.
