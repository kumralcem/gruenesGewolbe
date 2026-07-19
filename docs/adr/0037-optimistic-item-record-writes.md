# Optimistic Item Record Writes

Structured item editing uses optimistic writes and refuses an ordinary save when the canonical item record has changed since it was loaded. The app surfaces an Item Record Conflict and requires the user to reload the external version or deliberately overwrite it, preserving direct Markdown editing without allowing the UI to erase concurrent changes silently.
