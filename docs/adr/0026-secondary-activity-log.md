# Secondary Activity Log

The vault may include an append-only activity log for captures, imports, enrichment runs, errors, index rebuilds, and AI costs. The log is secondary audit and debugging material rather than an event store: item records and preserved files remain canonical, and the vault must not depend on replaying the log to be understood or rebuilt.
