# Embed Compact Item Records First

Vector search should initially embed compact item records rather than full source copies. This keeps retrieval cheaper and less noisy by focusing on titles, creators, tags, saving reasons, summaries, key extracted text, and generated captions; if retrieval later misses important material, the app can add selective embeddings for full source copies or specific preserved excerpts as a rebuildable enrichment.
