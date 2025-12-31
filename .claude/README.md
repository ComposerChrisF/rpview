# Claude AI Assistant Instructions

## CRITICAL: Git Commit Policy

**⚠️ NEVER commit changes to git without explicit user permission ⚠️**

### Rules for Git Commits

1. **DO NOT** automatically commit changes after making code modifications
2. **DO NOT** commit changes as part of completing a task
3. **ALWAYS** wait for the user to explicitly request a commit with phrases like:
   - "commit these changes"
   - "please commit"
   - "git commit"
   - "save to git"

4. **Exception**: The user may ask you to "commit changes" as part of their initial request. Only in this case should you commit without asking again.

### Correct Workflow

✅ **Good**:
```
User: "Add a feature to do X"
Assistant: [makes changes]
Assistant: "I've implemented feature X. The changes are ready. Would you like me to commit them?"
User: "Yes, please commit"
Assistant: [commits changes]
```

✅ **Also Good** (user requests commit upfront):
```
User: "Add feature X and commit the changes"
Assistant: [makes changes and commits]
```

❌ **Bad**:
```
User: "Add a feature to do X"
Assistant: [makes changes]
Assistant: [commits changes without asking]  ← WRONG!
```

### Why This Matters

- Users may want to review changes before committing
- Users may want to make additional modifications
- Users may want to write their own commit messages
- Users may be working in a branch and have specific workflow requirements

### Summary

**Always assume NO commit unless the user explicitly asks for it.**
