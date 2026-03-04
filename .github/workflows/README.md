# GitHub Actions Workflows

## Claude Code Review

This repository has two Claude Code Review workflows to help maintain code quality.

### 🤖 Auto Review (`claude-code-review-auto.yml`)

**Triggers**: Automatically when a PR is first opened

**Purpose**: Provides an initial code review for new PRs

**Features**:
- Runs only once when PR is created (not on every commit)
- Reviews code quality, potential bugs, performance, and security
- Uses repository's `CLAUDE.md` for style guidelines

**Can be customized to**:
- Only review PRs from specific contributors
- Only review PRs targeting certain branches
- Only review specific file types

---

### 💬 On-Demand Review (`claude-code-review-on-demand.yml`)

**Triggers**: When someone comments `@Claude` on a PR

**Purpose**: Request additional reviews or re-reviews after changes

**How to use**:

```
# General review
@Claude review

# Focus on specific concerns
@Claude check for security issues

@Claude review performance

@Claude 请检查并发安全问题
```

**Features**:
- ✅ Full control - only runs when explicitly requested
- ✅ Can be triggered multiple times (e.g., after fixes)
- ✅ Reacts with emoji to show status (👀 → 👍 or 😕)
- ✅ Prevents duplicate concurrent reviews
- ✅ Can specify review focus in the comment

---

## Usage Examples

### Example 1: Initial PR Creation

1. Create a new PR
2. Auto review runs automatically
3. Claude posts review comments

### Example 2: Request Re-Review After Changes

1. PR author makes changes based on feedback
2. PR author comments: `@Claude please re-review`
3. Claude reviews the updated code

### Example 3: Specific Focus

```
@Claude focus on the new async functions - any race conditions?
```

Claude will pay special attention to async code and potential race conditions.

---

## Configuration

### Secrets Required

- `CLAUDE_CODE_OAUTH_TOKEN`: OAuth token for Claude Code
  - Set in repository Settings → Secrets and variables → Actions

### Customization Options

#### Disable Auto Review

To disable automatic reviews on PR creation, either:
1. Delete `.github/workflows/claude-code-review-auto.yml`, or
2. Add a conditional to only run for specific users:

```yaml
if: github.event.pull_request.author_association == 'FIRST_TIME_CONTRIBUTOR'
```

#### Restrict Who Can Trigger On-Demand Review

Edit `claude-code-review-on-demand.yml`:

```yaml
if: |
  github.event.issue.pull_request &&
  contains(github.event.comment.body, '@Claude') &&
  (github.event.comment.user.login == github.event.issue.user.login ||
   contains(fromJSON('["maintainer", "owner", "member"]'), github.event.comment.author_association))
```

This only allows PR authors and maintainers to trigger reviews.

#### Change Review Criteria

Modify the `prompt` section in either workflow file to customize what Claude focuses on.

---

## Troubleshooting

### Review Not Triggering

**For Auto Review**:
- Check if workflow is enabled in Actions tab
- Verify `CLAUDE_CODE_OAUTH_TOKEN` secret is set
- Check if conditional filters are blocking execution

**For On-Demand Review**:
- Make sure comment includes `@Claude` (case-insensitive)
- Verify comment is on a PR (not a regular issue)
- Check Actions tab for workflow run status

### Review Taking Too Long

- Large PRs may take longer to review
- Check Actions tab for progress
- Claude will react with 👀 to show it's working

### Review Failed

- Check Actions tab for error logs
- Verify permissions are correctly set
- React with 😕 emoji indicates failure

---

## Cost Management

To manage GitHub Actions and Claude API costs:

1. **Disable auto-review** for all PRs and rely only on on-demand
2. **Filter by contributor** - only auto-review for new/junior developers
3. **Add file path filters** - only review specific directories:

```yaml
on:
  pull_request:
    types: [opened]
    paths:
      - 'src/**/*.ts'
      - 'src/**/*.tsx'
```

---

## Related Documentation

- [Claude Code Documentation](https://docs.claude.com/en/docs/claude-code)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- Repository `CLAUDE.md` - Style guidelines used by Claude
