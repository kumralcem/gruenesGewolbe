# Open Vaults Without Background Services

Opening an existing vault should not require a hidden app database, daemon, or background process to keep the vault functional. The app should treat vault files as canonical, detect missing or stale derived indexes on open, and rebuild or update them as needed; if full startup indexing becomes too slow, the solution should be lazy or incremental indexing rather than making the vault depend on a continuously running service.
