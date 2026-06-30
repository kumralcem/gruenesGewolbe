# OpenAI First Provider Boundary

The first AI implementation should use OpenAI because it is sufficient to start and avoids adding provider-routing complexity immediately. AI calls should still sit behind a provider boundary so later support for OpenRouter, cheaper third-party models, or local models can be added without changing the vault format or archive workflows.
