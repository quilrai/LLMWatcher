# Extracting Protobuf Definitions from Cursor

## Location

```
/Applications/Cursor.app/Contents/Resources/app/out/vs/workbench/workbench.desktop.main.js
```

## Method

### 1. Find message type names
```bash
grep -oE 'typeName[=:]"aiserver\.v1\.[^"]+' workbench.desktop.main.js | sort -u
```

### 2. Extract field definitions for a specific message
```bash
python3 -c "
with open('workbench.desktop.main.js', 'r') as f:
    content = f.read()
idx = content.find('StreamUnifiedChatRequest\"')
print(content[idx-200:idx+800])
"
```

### 3. Field definition format
Fields appear as:
```javascript
{no:1,name:"text",kind:"scalar",T:9}
{no:2,name:"type",kind:"enum",T:v.getEnumType(Va)}
{no:3,name:"attached_code_chunks",kind:"message",T:Oce,repeated:!0}
```

- `no`: Field number
- `name`: Field name
- `kind`: `scalar`, `enum`, or `message`
- `T`: Type (9=string, 8=bool, 5=int32, 1=double)
- `repeated:!0`: Array field

## Key Messages

| Message | Purpose |
|---------|---------|
| `StreamUnifiedChatRequest` | Main chat request |
| `StreamUnifiedChatResponseWithTools` | Chat response wrapper |
| `ConversationMessage` | Individual message (has `text` field) |
| `ConversationMessage.CodeChunk` | Attached code snippets |
| `ModelDetails` | Model name, API key, settings |

## Scalar Type Codes

| Code | Type |
|------|------|
| 1 | double |
| 5 | int32 |
| 8 | bool |
| 9 | string |
| 12 | bytes |
| 13 | uint32 |
