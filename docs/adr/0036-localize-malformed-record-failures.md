# Localize Malformed Item Record Failures

A malformed item record must not prevent its vault from opening or valid saved items from being browsed and searched. The app leaves the canonical file untouched, reports a Vault Problem with its path and parse error, and skips only that record during browsing and index rebuild so direct file editing cannot make the entire archive unavailable.
