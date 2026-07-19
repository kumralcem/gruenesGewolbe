# Best-Effort Import Runs

Paintings import processes supported files independently rather than as one atomic transaction. Successful saved items remain when another file fails, while skipped files, duplicate candidates, and failures appear in the import summary and failures are appended to the Activity Log; this favors progress and auditability for large personal collections over costly rollback behavior.
