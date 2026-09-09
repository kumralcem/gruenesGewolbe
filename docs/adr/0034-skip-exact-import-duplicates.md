# Skip Exact Import Duplicates by Default

Paintings import skips files whose content fingerprints exactly match preserved files already in the vault, while allowing an explicit import-anyway override. This narrows the earlier nonblocking duplicate behavior to ambiguous Duplicate Candidates: exact byte identity should not flood a re-run Import Run with redundant saved items and review work.
