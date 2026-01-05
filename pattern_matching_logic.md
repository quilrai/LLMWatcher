# Pattern Matching Logic

## Pattern Types

| Type | Behavior |
|------|----------|
| **Keyword** | Case-insensitive literal match. Input is escaped and wrapped with `(?i)`. |
| **Regex** | Raw regex pattern, case-sensitive by default. |

## Matching Flow

1. **Find matches** - Run all positive patterns against the text
2. **Context check** - For each match, extract a context window (30 chars before + match + 30 chars after)
3. **Negative filtering** - If any negative pattern matches within the context window, exclude that specific match
4. **Unique chars filter** - Reject matches with fewer than `min_unique_chars` distinct characters
5. **Deduplicate** - Remove duplicate matches
6. **Occurrence threshold** - Only return matches if total count >= `min_occurrences`

## Negative Patterns (Context-Aware)

Negative patterns don't exclude the entire pattern group—they exclude **individual matches** based on surrounding context.

```
Text: "testing key: sk-test123 ... production key: sk-prod456"
Pattern: sk-[a-z0-9]+
Negative: test

Result: Only "sk-prod456" matches
        (sk-test123 excluded because "test" appears within 30 chars)
```

## Validation Filters

| Filter | Purpose |
|--------|---------|
| **Min Unique Chars** | Rejects low-entropy matches (e.g., "aaaa" has 1 unique char) |
| **Min Occurrences** | Requires N matches before flagging (reduces single false positives) |
