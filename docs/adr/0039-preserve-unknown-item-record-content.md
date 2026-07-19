# Preserve Unknown Item Record Content

Structured item-record writes may update known frontmatter fields and app-owned Markdown sections, but must round-trip unknown frontmatter and user-authored Markdown sections unchanged. Direct file editing remains trustworthy only if the app does not discard content it does not understand when saving another field.
